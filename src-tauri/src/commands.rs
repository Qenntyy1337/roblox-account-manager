//! Tauri-команды фронтенда: CRUD аккаунтов, запуск/завершение клиентов,
//! настройки, диалоги, экспорт логов, presence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State};

use crate::crypto;
use crate::monitor::{self, Track};
use crate::roblox;
use crate::store::{self, Account, Config, Settings};
use crate::winproc;

pub struct AppState {
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    pub config: Mutex<Config>,
    pub instances: Mutex<HashMap<String, Track>>,
    /// Сериализует запуски: следующий старт начинается только после паузы,
    /// чтобы предыдущий клиент успел создать свои синглтон-объекты —
    /// иначе два одновременно стартующих клиента убивают друг друга.
    pub launch_lock: tauri::async_runtime::Mutex<()>,
}

/* --------------------------- DTO --------------------------- */

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto {
    pub settings: Settings,
    pub accounts: Vec<Account>,
    pub instances: Vec<monitor::InstanceDto>,
    pub roblox_path: Option<String>,
    pub app_version: &'static str,
}

#[derive(Serialize, Clone)]
pub struct LaunchDto {
    pub pid: u32,
    /// autologin | plain — как был произведён запуск.
    pub mode: String,
}

#[derive(Serialize)]
pub struct ValidateDto {
    pub ok: bool,
    pub username: Option<String>,
    /// Машинный ключ причины отказа: invalid | network | rate_limited.
    pub reason: Option<String>,
    /// Техническая деталь для подсказки/лога.
    pub detail: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct AccountInputDto {
    pub username: Option<String>,
    pub cookie: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Success,
}

#[derive(Serialize, Clone)]
pub struct LogEntryDto {
    pub id: String,
    pub ts: i64,
    pub level: LogLevel,
    pub account: Option<String>,
    pub message: String,
}

/* ------------------------- Логирование ------------------------- */

pub fn log_event(app: &AppHandle, level: &str, account: Option<&str>, message: &str) {
    use tauri::Emitter;
    let level = match level {
        "error" => LogLevel::Error,
        "warning" => LogLevel::Warning,
        "debug" => LogLevel::Debug,
        "success" => LogLevel::Success,
        _ => LogLevel::Info,
    };
    let entry = LogEntryDto {
        id: uuid::Uuid::new_v4().to_string(),
        ts: chrono::Utc::now().timestamp_millis(),
        level,
        account: account.map(|s| s.to_string()),
        message: message.to_string(),
    };
    let _ = app.emit("log", &entry);
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn persist(state: &AppState, cfg: &Config) -> Result<(), String> {
    store::save(&state.config_path, cfg)
}

/// Слить накопленное время сессий (Track.acc_secs) в аккаунты и сохранить
/// на диск. Вызывается из монитора раз в ~30 с и при завершении клиента.
pub fn flush_playtime(state: &AppState) {
    let now = chrono::Utc::now().timestamp_millis();
    let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
    let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
    let mut dirty = false;
    for (id, t) in map.iter_mut() {
        if t.acc_secs > 0 {
            if let Some(a) = cfg.accounts.iter_mut().find(|a| &a.id == id) {
                a.playtime_secs += t.acc_secs;
                a.last_played = Some(now);
                dirty = true;
            }
            t.acc_secs = 0;
        }
    }
    if dirty {
        let _ = store::save(&state.config_path, &cfg);
    }
}

/* ------------------------- Команды ------------------------- */

#[tauri::command]
pub fn get_bootstrap(state: State<AppState>) -> BootstrapDto {
    let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let instances = monitor::snapshot(state.inner());
    let roblox_path = cfg
        .settings
        .roblox_path
        .clone()
        .or_else(winproc::detect_client_path);
    BootstrapDto {
        settings: cfg.settings,
        accounts: cfg.accounts,
        instances,
        roblox_path,
        app_version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.settings = settings;
        persist(state.inner(), &cfg)?;
    }
    log_event(&app, "info", None, "Settings updated");
    Ok(())
}

#[tauri::command]
pub async fn add_account(
    app: AppHandle,
    state: State<'_, AppState>,
    input: AccountInputDto,
) -> Result<Account, String> {
    let cookie = roblox::normalize_cookie(&input.cookie.clone().unwrap_or_default());
    if cookie.is_empty() {
        return Err("Cookie .ROBLOSECURITY обязателен".into());
    }

    // Сеть — в блокирующем пуле
    let probe = {
        let cookie = cookie.clone();
        tauri::async_runtime::spawn_blocking(move || match roblox::validate_cookie_ex(&cookie) {
            Ok(user) => {
                let avatar = roblox::avatar_url(user.id);
                (Some(user), avatar, None)
            }
            Err(e) => (None, None, Some(e)),
        })
        .await
        .map_err(|e| e.to_string())?
    };
    let (user, avatar, fail) = probe;
    if let Some(e) = &fail {
        log_event(&app, "error", None, &format!("Cookie validation failed: {}", e.detail()));
    }

    let username = input
        .username
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            user.as_ref()
                .map(|u| u.name.clone())
                .unwrap_or_else(|| format!("Player_{}", rand4()))
        });

    let acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.clone(),
        user_id: user.as_ref().map(|u| u.id),
        avatar_url: avatar,
        cookie_ok: user.is_some(),
        note: input.note.unwrap_or_default().trim().to_string(),
        created_at: now_ms(),
        playtime_secs: 0,
        last_played: None,
        cookie_enc: crypto::encrypt_cookie(&cookie),
    };

    {
        let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.accounts.push(acc.clone());
        persist(state.inner(), &cfg)?;
    }

    log_event(
        &app,
        if acc.cookie_ok { "success" } else { "warning" },
        Some(&username),
        &format!(
            "Account \"{}\" added (cookie {})",
            username,
            if acc.cookie_ok { "verified" } else { "not verified" }
        ),
    );
    Ok(acc)
}

#[tauri::command]
pub async fn update_account(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: AccountInputDto,
) -> Result<Account, String> {
    let cookie_opt = input
        .cookie
        .map(|c| roblox::normalize_cookie(&c))
        .filter(|s| !s.is_empty());

    let probe = if let Some(cookie) = cookie_opt.clone() {
        Some(
            tauri::async_runtime::spawn_blocking(move || match roblox::validate_cookie_ex(&cookie) {
                Ok(user) => {
                    let avatar = roblox::avatar_url(user.id);
                    (Some(user), avatar, None)
                }
                Err(e) => (None, None, Some(e)),
            })
            .await
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    let updated = {
        let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        let acc = cfg
            .accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or("Аккаунт не найден")?;
        if let Some(u) = input.username.filter(|s| !s.trim().is_empty()) {
            acc.username = u.trim().to_string();
        }
        if let Some(n) = input.note {
            acc.note = n.trim().to_string();
        }
        if let Some(cookie) = cookie_opt {
            acc.cookie_enc = crypto::encrypt_cookie(&cookie);
            let mut fail_reason = None;
            if let Some((user, avatar, err)) = probe {
                match (user, err) {
                    (Some(u), _) => {
                        acc.cookie_ok = true;
                        acc.user_id = Some(u.id);
                        acc.avatar_url = avatar;
                    }
                    (None, Some(e)) => {
                        acc.cookie_ok = false;
                        fail_reason = Some(e);
                    }
                    (None, None) => acc.cookie_ok = false,
                }
            } else {
                acc.cookie_ok = false;
            }
            if let Some(e) = fail_reason {
                let uname = acc.username.clone();
                log_event(
                    &app,
                    "error",
                    Some(&uname),
                    &format!("Cookie validation failed: {}", e.detail()),
                );
            }
        }
        acc.clone()
    };

    persist(state.inner(), &state.config.lock().unwrap_or_else(|e| e.into_inner()))?;
    log_event(
        &app,
        "success",
        Some(&updated.username),
        &format!("Account \"{}\" updated", updated.username),
    );
    Ok(updated)
}

#[tauri::command]
pub fn delete_account(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    // Слить накопленное время до удаления аккаунта
    flush_playtime(state.inner());
    let username;
    {
        let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        let Some(pos) = cfg.accounts.iter().position(|a| a.id == id) else {
            return Err("Аккаунт не найден".into());
        };
        let acc = cfg.accounts.remove(pos);
        username = acc.username.clone();
    }
    // Остановить запущенный клиент, если был
    {
        let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = map.remove(&id) {
            let _ = winproc::terminate(t.pid);
        }
    }
    persist(state.inner(), &state.config.lock().unwrap_or_else(|e| e.into_inner()))?;
    log_event(&app, "warning", Some(&username), &format!("Account \"{username}\" deleted"));
    Ok(())
}

/// Пауза после открытия клиента (внутри launch-замка): следующий запуск
/// обязан начаться, когда этот клиент уже создал свои синглтон-объекты —
/// тогда процедура запуска закроет их заранее и оба клиента выживут.
async fn spacing_wait() {
    let _ = tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(2500));
    })
    .await;
}

/// Краткая диагностика скана для DEBUG-лога.
fn scan_note(scan: &winproc::SingletonScan) -> String {
    let mut s = format!(
        "process open {}, handles {}, event/mutant {}, closed {}",
        scan.open_ok, scan.in_process, scan.typed, scan.closed
    );
    if !scan.names.is_empty() {
        s.push_str(", names: ");
        s.push_str(&scan.names.join(" | "));
    }
    s
}

#[tauri::command]
pub async fn launch_account(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<LaunchDto, String> {
    /* ---------- Резерв слота: читаем конфиг ДО вставки ---------- */
    let (path, isolate, close_single, auto_login, username, cookie_ok, cookie_enc) = {
        let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let acc = cfg
            .accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or("Аккаунт не найден")?;
        let path = cfg
            .settings
            .roblox_path
            .clone()
            .or_else(winproc::detect_client_path)
            .ok_or("RobloxPlayerBeta.exe не найден — укажите путь в настройках")?;
        (
            path,
            cfg.settings.isolate_data,
            cfg.settings.close_singleton,
            cfg.settings.auto_login,
            acc.username.clone(),
            acc.cookie_ok,
            acc.cookie_enc.clone(),
        )
    };
    let intended_mode = if auto_login && cookie_ok { "autologin" } else { "plain" };

    /* ---------- Антидабл-клик: занимаем слот атомарно ---------- */
    {
        let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
        if map.contains_key(&id) {
            return Err("Клиент этого аккаунта уже запущен".into());
        }
        let now = now_ms();
        map.insert(
            id.clone(),
            Track {
                pid: 0,
                // reserved — слот занят, но реальный запуск ещё не начался:
                // монитор такие треки игнорирует (не прицепляет процессы)
                status: "reserved".into(),
                started_at: now,
                last_title: None,
                mode: intended_mode.into(),
                acc_secs: 0,
                last_tick_ms: now,
                ticks: 0,
                dead_since: None,
            },
        );
    }

    /* ---------- Сериализация: один запуск за раз ---------- */
    let _launch_guard = state.launch_lock.lock().await;

    /* ---------- Автологин: auth ticket + протокол roblox-player ---------- */
    if auto_login && cookie_ok {
        let ticket = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
            let cookie = crypto::decrypt_cookie(&cookie_enc)
                .ok_or("не удалось расшифровать сохранённый cookie")?;
            roblox::auth_ticket(&cookie)
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);

        match ticket {
            Ok(ticket) => {
                // КРИТИЧНО: закрыть хэндлы ДО открытия протокола,
                // иначе новый процесс убьёт старый клиент сигналом.
                if close_single {
                    close_singletons_pre(&app, &username).await;
                }
                let url = roblox::player_protocol_url(&ticket);
                match winproc::open_protocol(&url) {
                    Ok(()) => {
                        let now = now_ms();
                        {
                            let mut map =
                                state.instances.lock().unwrap_or_else(|e| e.into_inner());
                            map.insert(
                                id.clone(),
                                Track {
                                    // Клиент ещё не появился — монитору найти самому
                                    pid: 0,
                                    status: "launching".into(),
                                    started_at: now,
                                    last_title: None,
                                    mode: "autologin".into(),
                                    acc_secs: 0,
                                    last_tick_ms: now,
                                    ticks: 0,
                                    dead_since: None,
                                },
                            );
                        }
                        log_event(
                            &app,
                            "success",
                            Some(&username),
                            &format!(
                                "Auth ticket issued — launching client via roblox-player protocol (autologin) for \"{username}\""
                            ),
                        );
                        if close_single {
                            spawn_close_singletons(&app, &username);
                        }
                        spacing_wait().await;
                        return Ok(LaunchDto { pid: 0, mode: "autologin".into() });
                    }
                    Err(err) => {
                        log_event(
                            &app,
                            "warning",
                            Some(&username),
                            &format!("Autologin failed ({err}) — falling back to plain launch"),
                        );
                    }
                }
            }
            Err(err) => {
                log_event(
                    &app,
                    "warning",
                    Some(&username),
                    &format!(
                        "Auth ticket request failed ({err}) — falling back to plain launch"
                    ),
                );
            }
        }
    } else if auto_login && !cookie_ok {
        log_event(
            &app,
            "warning",
            Some(&username),
            "Autologin skipped: cookie is not verified — launching without login",
        );
    }

    /* ---------- Обычный запуск: напрямую RobloxPlayerBeta.exe ---------- */
    // Закрыть синглтон-хэндлы ДО запуска, иначе новый клиент
    // просигналит ROBLOX_singletonEvent и старый Roblox закроется.
    if close_single {
        close_singletons_pre(&app, &username).await;
    }

    let data_dir = state.data_dir.clone();
    let id2 = id.clone();
    let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<u32, String> {
        let lad = if isolate {
            let dir = data_dir.join("instances").join(&id2);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Some(dir)
        } else {
            None
        };
        winproc::spawn_roblox(&path, lad.as_deref())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);

    let pid = match spawned {
        Ok(p) => p,
        Err(e) => {
            // Слот резерва не нужен — запуск не удался
            state
                .instances
                .lock()
                .unwrap_or_else(|e2| e2.into_inner())
                .remove(&id);
            return Err(e);
        }
    };

    {
        let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        map.insert(
            id.clone(),
            Track {
                pid,
                status: "launching".into(),
                started_at: now,
                last_title: None,
                mode: "plain".into(),
                acc_secs: 0,
                last_tick_ms: now,
                ticks: 0,
                dead_since: None,
            },
        );
    }

    log_event(
        &app,
        "info",
        Some(&username),
        &format!("Launching RobloxPlayerBeta.exe (pid {pid}) for \"{username}\""),
    );

    // Обход синглтон-хэндла: закрываем ROBLOX_single* во всех клиентах
    if close_single {
        spawn_close_singletons(&app, &username);
    }

    spacing_wait().await;
    Ok(LaunchDto { pid, mode: "plain".into() })
}

/// Синхронное закрытие синглтон-хэндлов ДО запуска нового клиента.
/// Vanilla Roblox при старте «сигналит» событие ROBLOX_singletonEvent,
/// и старый клиент закрывается. Если разрушить объект заранее (закрыв
/// все его хэндлы), новому клиенту некого сигналить — оба продолжат
/// работать. Это и есть обход одиночного режима (multi-instance).
async fn close_singletons_pre(app: &AppHandle, username: &str) {
    let app2 = app.clone();
    let username2 = username.to_string();
    let closed = tauri::async_runtime::spawn_blocking(move || -> usize {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut total = 0usize;
        let mut clear_rounds = 0usize;
        let mut first_round = true;
        loop {
            if std::time::Instant::now() >= deadline {
                break;
            }
            let pids = winproc::roblox_pids();
            if pids.is_empty() {
                break; // ни одного клиента Roblox — путь свободен
            }
            let mut round = 0usize;
            for pid in pids {
                if !winproc::is_alive(pid) {
                    continue;
                }
                let scan = winproc::close_singleton_handles(pid);
                if first_round {
                    log_event(
                        &app2,
                        "debug",
                        Some(&username2),
                        &format!("Singleton scan pid {pid}: {}", scan_note(&scan)),
                    );
                }
                if scan.closed > 0 {
                    round += scan.closed;
                    log_event(
                        &app2,
                        "success",
                        Some(&username2),
                        &format!("Closed {} Roblox singleton handle(s) (pid {pid})", scan.closed),
                    );
                }
            }
            first_round = false;
            total += round;
            if round == 0 {
                clear_rounds += 1;
                if clear_rounds >= 6 {
                    break; // ~1.8 с подряд чисто — путь свободен
                }
            } else {
                clear_rounds = 0;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        total
    })
    .await
    .unwrap_or(0);

    if closed > 0 {
        log_event(
            app,
            "success",
            Some(username),
            &format!("Singleton handles cleared before launch ({closed}) — old clients stay alive"),
        );
    }
}

/// Обход синглтон-хэндла: закрываем ROBLOX_single* во всех клиентах
/// (фоновый поток на 12 с после каждого запуска — страховка).
fn spawn_close_singletons(app: &AppHandle, username: &str) {
    let app2 = app.clone();
    let username2 = username.to_string();
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
        let mut closed_any = false;
        let mut first_round = true;
        while std::time::Instant::now() < deadline {
            for pid in winproc::roblox_pids() {
                if !winproc::is_alive(pid) {
                    continue;
                }
                let scan = winproc::close_singleton_handles(pid);
                if first_round {
                    log_event(
                        &app2,
                        "debug",
                        Some(&username2),
                        &format!("Singleton sweep pid {pid}: {}", scan_note(&scan)),
                    );
                }
                if scan.closed > 0 {
                    closed_any = true;
                    log_event(
                        &app2,
                        "success",
                        Some(&username2),
                        &format!("Closed {} Roblox singleton handle(s) (pid {pid})", scan.closed),
                    );
                }
            }
            first_round = false;
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        if !closed_any {
            log_event(
                &app2,
                "debug",
                Some(&username2),
                "No Roblox singleton handles found (path is clear)",
            );
        }
    });
}

#[tauri::command]
pub fn terminate_instance(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    flush_playtime(state.inner());
    let pid = {
        let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&id).map(|t| t.pid)
    };
    if let Some(pid) = pid {
        winproc::terminate(pid)?;
        let username = account_name_by_id(state.inner(), &id);
        log_event(
            &app,
            "warning",
            username.as_deref(),
            &format!("Client terminated (pid {pid})"),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn close_all(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    flush_playtime(state.inner());
    let pids: Vec<(String, u32)> = {
        let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
        map.drain().map(|(k, v)| (k, v.pid)).collect()
    };
    for (id, pid) in pids {
        let _ = winproc::terminate(pid);
        let username = account_name_by_id(state.inner(), &id);
        log_event(&app, "warning", username.as_deref(), &format!("Client terminated (pid {pid})"));
    }
    Ok(())
}

#[tauri::command]
pub async fn validate_cookie(app: AppHandle, cookie: String) -> ValidateDto {
    let cleaned = roblox::normalize_cookie(&cookie);
    let res = tauri::async_runtime::spawn_blocking(move || roblox::validate_cookie_ex(&cleaned))
        .await
        .unwrap_or_else(|e| Err(roblox::CookieCheckError::Network(e.to_string())));

    if let Err(e) = &res {
        log_event(
            &app,
            "error",
            None,
            &format!("Cookie validation failed: {}", e.detail()),
        );
    }

    match res {
        Ok(user) => ValidateDto {
            ok: true,
            username: Some(user.name),
            reason: None,
            detail: None,
        },
        Err(e) => ValidateDto {
            ok: false,
            username: None,
            reason: Some(e.reason().to_string()),
            detail: Some(e.detail()),
        },
    }
}

#[tauri::command]
pub fn detect_roblox_path(state: State<AppState>) -> Option<String> {
    let found = winproc::detect_client_path();
    if let Some(p) = &found {
        let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.settings.roblox_path = Some(p.clone());
        let _ = persist(state.inner(), &cfg);
    }
    found
}

#[tauri::command]
pub async fn pick_file() -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .unwrap_or(None)
}

#[tauri::command]
pub async fn export_logs(app: AppHandle, content: String) -> Result<bool, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        rfd::FileDialog::new()
            .add_filter("Log files", &["log", "txt"])
            .set_file_name(format!("ram-console-{stamp}.log").as_str())
            .save_file()
            .map(|p| (p.to_string_lossy().into_owned(), std::fs::write(p, content)))
    })
    .await
    .map_err(|e| e.to_string())?;

    match result {
        Some((path, Ok(()))) => {
            log_event(&app, "success", None, &format!("Logs exported to {path}"));
            Ok(true)
        }
        Some((_, Err(e))) => Err(e.to_string()),
        None => Ok(false), // диалог закрыт
    }
}

#[tauri::command]
pub fn open_path(state: State<AppState>, path: String) -> Result<(), String> {
    let target: PathBuf = if path == "__appdata__" {
        state.data_dir.clone()
    } else {
        PathBuf::from(path)
    };
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_presence(state: State<'_, AppState>) -> Result<HashMap<u64, u8>, String> {
    // Собираем (user_id, cookie) проверенных аккаунтов
    let pairs: Vec<(u64, String)> = {
        let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
        cfg.accounts
            .iter()
            .filter(|a| a.cookie_ok && a.user_id.is_some())
            .filter_map(|a| {
                crypto::decrypt_cookie(&a.cookie_enc).map(|c| (a.user_id.unwrap(), c))
            })
            .collect()
    };
    if pairs.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<u64> = pairs.iter().map(|(id, _)| *id).collect();
    let cookie = pairs[0].1.clone();
    tauri::async_runtime::spawn_blocking(move || {
        roblox::fetch_presence(&cookie, &ids).unwrap_or_default()
    })
    .await
    .map_err(|e| e.to_string())
}

/* ------------------------- Утилиты ------------------------- */

fn rand4() -> u32 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    1000 + (t % 9000)
}

fn account_name_by_id(state: &AppState, id: &str) -> Option<String> {
    let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
    cfg.accounts
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.username.clone())
}
