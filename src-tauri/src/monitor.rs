//! Фоновый монитор запущенных клиентов: жив ли процесс, заголовок окна
//! (в меню / в игре), события выхода и крашей, накопление наработки.
//! Публикует снапшот "instances".

use serde::Serialize;
use std::collections::HashSet;
use std::time::Duration;

use crate::commands::{flush_playtime, log_event, AppState};
use crate::winproc;

#[derive(Debug, Clone)]
pub struct Track {
    pub pid: u32,
    pub status: String,
    pub started_at: i64,
    pub last_title: Option<String>,
    /// Накопленное за текущую сессию время (сек) с последнего флаша.
    pub acc_secs: u64,
    /// Момент предыдущего тика (unix ms) — для расчёта дельты.
    pub last_tick_ms: i64,
    /// Тиков с последнего сохранения наработки на диск.
    pub ticks: u32,
}

#[derive(Serialize, Clone)]
pub struct InstanceDto {
    pub account_id: String,
    pub pid: u32,
    pub status: String,
    pub started_at: i64,
    pub title: Option<String>,
    /// Суммарная наработка: сохранённая + текущая сессия, сек.
    pub playtime_secs: u64,
}

pub fn snapshot(state: &AppState) -> Vec<InstanceDto> {
    let saved: std::collections::HashMap<String, u64> = {
        let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.accounts
            .iter()
            .map(|a| (a.id.clone(), a.playtime_secs))
            .collect()
    };
    let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
    map.iter()
        .map(|(id, t)| InstanceDto {
            account_id: id.clone(),
            pid: t.pid,
            status: t.status.clone(),
            started_at: t.started_at,
            title: t.last_title.clone(),
            playtime_secs: saved.get(id).copied().unwrap_or(0) + t.acc_secs,
        })
        .collect()
}

pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(2000));
            tick(&app);
        }
    });
}

/// «Adopt Me - Roblox» -> «Adopt Me»; обычные названия остаются как есть.
fn game_label(title: &str) -> String {
    let t = title.trim();
    let lower = t.to_ascii_lowercase();
    for sep in [" - roblox", " – roblox", " | roblox"] {
        if let Some(pos) = lower.find(sep) {
            let head = t[..pos].trim();
            if !head.is_empty() {
                return head.to_string();
            }
        }
    }
    t.to_string()
}

fn tick(app: &tauri::AppHandle) {
    use tauri::Manager;

    let state = app.state::<AppState>();
    let ids: Vec<String> = {
        let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
        map.keys().cloned().collect()
    };
    if ids.is_empty() {
        return;
    }

    // Сохранённая наработка аккаунтов — для суммы в DTO
    let saved_playtime: std::collections::HashMap<String, u64> = {
        let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.accounts
            .iter()
            .map(|a| (a.id.clone(), a.playtime_secs))
            .collect()
    };

    let mut changed = false;
    let mut snapshot = Vec::new();

    for id in ids {
        let track = {
            let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            map.get(&id).cloned()
        };
        let Some(mut t) = track else { continue };
        let now = chrono::Utc::now().timestamp_millis();

        // pid == 0 — автологин через протокол: клиент ещё не появился,
        // ищем свежий процесс Roblox, стартовавший после запуска.
        if t.pid == 0 {
            if now - t.started_at > 90_000 {
                let username = account_name(&state, &id);
                log_event(
                    app,
                    "error",
                    username.as_deref(),
                    "Autologin client did not start within 90 s — giving up",
                );
                {
                    let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                    map.remove(&id);
                }
                changed = true;
                continue;
            }
            let tracked: HashSet<u32> = {
                let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                map.values().map(|x| x.pid).filter(|p| *p != 0).collect()
            };
            let candidate = winproc::roblox_pids()
                .into_iter()
                .filter(|p| !tracked.contains(p))
                .filter_map(|p| winproc::process_start_time(p).map(|st| (p, st)))
                .filter(|(_p, st)| *st >= t.started_at - 1500)
                .max_by_key(|(_p, st)| *st);
            if let Some((new_pid, _)) = candidate {
                let username = account_name(&state, &id);
                {
                    let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                    map.insert(
                        id.clone(),
                        Track {
                            pid: new_pid,
                            status: "launching".into(),
                            started_at: t.started_at,
                            last_title: None,
                            acc_secs: t.acc_secs,
                            last_tick_ms: now,
                            ticks: 0,
                        },
                    );
                }
                changed = true;
                log_event(
                    app,
                    "success",
                    username.as_deref(),
                    &format!("Client attached after autologin (pid {new_pid})"),
                );
            }
            continue;
        }

        if !winproc::is_alive(t.pid) {
            // Клиент закрылся. Roblox часто перезапускает сам себя при старте:
            // подхватываем новый процесс, если он появился сразу после запуска.
            let young = now - t.started_at < 45_000;
            if young {
                let tracked: HashSet<u32> = {
                    let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                    map.values().map(|x| x.pid).collect()
                };
                let untracked: Vec<u32> = winproc::roblox_pids()
                    .into_iter()
                    .filter(|p| !tracked.contains(p) && *p != t.pid)
                    .collect();
                if untracked.len() == 1 {
                    let new_pid = untracked[0];
                    let username = account_name(&state, &id);
                    {
                        let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                        map.insert(
                            id.clone(),
                            Track {
                                pid: new_pid,
                                status: "launching".into(),
                                started_at: chrono::Utc::now().timestamp_millis(),
                                last_title: None,
                                acc_secs: t.acc_secs,
                                last_tick_ms: now,
                                ticks: 0,
                            },
                        );
                    }
                    changed = true;
                    log_event(
                        app,
                        "info",
                        username.as_deref(),
                        &format!(
                            "Client relaunched itself (pid {} -> pid {}), tracking continues",
                            t.pid, new_pid
                        ),
                    );
                    continue;
                }
            }

            // Реальный выход: различаем нормальное закрытие и краш по коду.
            let code = winproc::exit_code(t.pid);
            // Добираем время с последнего тика, чтобы ничего не потерять
            let tail_ms = (now - t.last_tick_ms).clamp(0, 10_000);
            t.acc_secs += (tail_ms / 1000) as u64;
            let username = {
                let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
                cfg.accounts
                    .iter()
                    .find(|a| a.id == id)
                    .map(|a| a.username.clone())
            };
            // Финальный флаш наработки этой сессии перед удалением Track.
            {
                let mut cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
                if t.acc_secs > 0 {
                    if let Some(a) = cfg.accounts.iter_mut().find(|a| a.id == id) {
                        a.playtime_secs += t.acc_secs;
                        a.last_played = Some(now);
                    }
                }
                let _ = crate::store::save(&state.config_path, &cfg);
            }
            let session_min = (now - t.started_at) / 60_000;
            let session_note = if session_min > 0 {
                format!(" (session {} min)", session_min)
            } else {
                String::new()
            };
            {
                let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                map.remove(&id);
            }
            changed = true;
            match code {
                Some(0) => log_event(
                    app,
                    "info",
                    username.as_deref(),
                    &format!(
                        "Client exited normally (pid {}){}{}",
                        t.pid,
                        session_note,
                        if t.acc_secs > 0 {
                            format!(
                                " — playtime +{} min",
                                t.acc_secs / 60
                            )
                        } else {
                            String::new()
                        }
                    ),
                ),
                Some(c) => {
                    let game = t.last_title.clone().unwrap_or_default();
                    let while_in = if game.is_empty() {
                        String::new()
                    } else {
                        format!(" while in game: \"{}\"", game)
                    };
                    log_event(
                        app,
                        "error",
                        username.as_deref(),
                        &format!(
                            "Client CRASHED (pid {}, exit code 0x{:08X}){}{}",
                            t.pid, c, while_in, session_note
                        ),
                    );
                }
                None => log_event(
                    app,
                    "info",
                    username.as_deref(),
                    &format!("Client exited (pid {}){}", t.pid, session_note),
                ),
            }
            continue;
        }

        // Клиент жив: копим наработку (дельта между тиками, с потолком 10 с).
        let delta_ms = (now - t.last_tick_ms).clamp(0, 10_000);
        t.acc_secs += (delta_ms / 1000) as u64;
        t.last_tick_ms = now;
        t.ticks = t.ticks.saturating_add(1);
        if t.ticks >= 15 {
            // Раз в ~30 с сохраняем наработку на диск.
            t.ticks = 0;
            flush_playtime(state.inner());
        }

        let raw_title = winproc::window_title(t.pid);
        let status = match &raw_title {
            Some(s)
                if !s.is_empty()
                    && s != "Roblox"
                    && s.to_ascii_lowercase().contains("roblox") =>
            {
                "ingame"
            }
            _ => "online",
        };

        // Чистое название игры (без суффикса " - Roblox") для UI и логов
        let title = raw_title.map(|s| {
            let lbl = game_label(&s);
            if lbl.is_empty() {
                s
            } else {
                lbl
            }
        });

        if t.status == "launching" {
            let username = account_name(&state, &id);
            log_event(
                app,
                "success",
                username.as_deref(),
                &format!(
                    "Client started (pid {}): \"{}\"",
                    t.pid,
                    title.as_deref().unwrap_or("Roblox")
                ),
            );
        } else if status == "ingame" && t.status != "ingame" {
            let username = account_name(&state, &id);
            log_event(
                app,
                "success",
                username.as_deref(),
                &format!("Joined game: {}", title.as_deref().unwrap_or("?")),
            );
        } else if status != "ingame" && t.status == "ingame" {
            let username = account_name(&state, &id);
            log_event(app, "info", username.as_deref(), "Left game — client back in menu");
        }

        t.status = status.to_string();
        t.last_title = title;

        {
            let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            map.insert(id.clone(), t.clone());
        }
        changed = true;

        let total_secs = saved_playtime.get(&id).copied().unwrap_or(0) + t.acc_secs;
        snapshot.push(InstanceDto {
            account_id: id,
            pid: t.pid,
            status: t.status,
            started_at: t.started_at,
            title: t.last_title,
            playtime_secs: total_secs,
        });
    }

    if changed {
        use tauri::Emitter;
        let _ = app.emit("instances", &snapshot);
    }
}

fn account_name(state: &AppState, id: &str) -> Option<String> {
    let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
    cfg.accounts
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.username.clone())
}
