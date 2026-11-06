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
    /// Как был запущен клиент: autologin | plain.
    pub mode: String,
    /// Накопленное за текущую сессию время (сек) с последнего флаша.
    pub acc_secs: u64,
    /// Момент предыдущего тика (unix ms) — для расчёта дельты.
    pub last_tick_ms: i64,
    /// Тиков с последнего сохранения наработки на диск.
    pub ticks: u32,
    /// Момент, когда процесс был впервые замечен мёртвым (unix ms).
    /// Пока < 8 с — ждём самоперезапуск клиента, не удаляя трек.
    pub dead_since: Option<i64>,
}

#[derive(Serialize, Clone)]
pub struct InstanceDto {
    pub account_id: String,
    pub pid: u32,
    pub status: String,
    pub started_at: i64,
    pub title: Option<String>,
    /// Как запущен клиент: autologin | plain.
    pub mode: Option<String>,
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
            mode: Some(t.mode.clone()),
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
    // PID, назначенные мёртвым трекам в этом тике (чтобы два трека
    // не забрали один и тот же перезапустившийся процесс).
    let mut taken_pids: HashSet<u32> = HashSet::new();

    /* --- Автологин-треки (pid == 0): глобальное назначение процессов ---
       Раньше каждый трек брал «самый свежий» процесс — при запуске двух
       аккаунтов подряд они могли обменяться клиентами. Теперь назначаем
       всем трекам за один проход: кто запустился раньше, тот получает
       процесс, родившийся раньше. */
    {
        let now = chrono::Utc::now().timestamp_millis();
        let mut launching: Vec<(String, Track)> = {
            let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            map.iter()
                // reserved — слот, занятый запуском до открытия клиента:
                // игнорируем, иначе прицепим чужой свежий процесс
                .filter(|(_, t)| t.pid == 0 && t.status == "launching")
                .map(|(k, t)| (k.clone(), t.clone()))
                .collect()
        };
        launching.sort_by_key(|(_, t)| t.started_at);

        if !launching.is_empty() {
            let tracked: HashSet<u32> = {
                let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                map.values().map(|x| x.pid).filter(|p| *p != 0).collect()
            };
            let mut candidates: Vec<(u32, i64)> = winproc::roblox_pids()
                .into_iter()
                .filter(|p| !tracked.contains(p))
                .filter_map(|p| winproc::process_start_time(p).map(|st| (p, st)))
                .collect();
            candidates.sort_by_key(|&(_, st)| st);

            let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            let mut taken: HashSet<u32> = HashSet::new();
            for (id, t) in launching {
                let pick = candidates
                    .iter()
                    .copied()
                    .find(|(p, st)| !taken.contains(p) && *st >= t.started_at - 1500);
                match pick {
                    Some((new_pid, _)) => {
                        taken.insert(new_pid);
                        let username = account_name(&state, &id);
                        map.insert(
                            id.clone(),
                            Track {
                                pid: new_pid,
                                status: "launching".into(),
                                started_at: t.started_at,
                                last_title: None,
                                mode: t.mode.clone(),
                                acc_secs: t.acc_secs,
                                last_tick_ms: now,
                                ticks: 0,
                                dead_since: None,
                            },
                        );
                        changed = true;
                        log_event(
                            app,
                            "success",
                            username.as_deref(),
                            &format!("Client attached after autologin (pid {new_pid})"),
                        );
                    }
                    None => {
                        if now - t.started_at > 90_000 {
                            let username = account_name(&state, &id);
                            map.remove(&id);
                            changed = true;
                            log_event(
                                app,
                                "error",
                                username.as_deref(),
                                "Autologin client did not start within 90 s — giving up",
                            );
                        }
                    }
                }
            }

            // Пока клиент не найден — показываем трек как «Запуск» в UI
            let pending: Vec<(String, Track)> = map
                .iter()
                .filter(|(_, t)| t.pid == 0 && t.status == "launching")
                .map(|(k, t)| (k.clone(), t.clone()))
                .collect();
            for (id, t) in pending {
                changed = true;
                let saved = saved_playtime.get(&id).copied().unwrap_or(0);
                snapshot.push(InstanceDto {
                    account_id: id,
                    pid: 0,
                    status: "launching".into(),
                    started_at: t.started_at,
                    title: None,
                    mode: Some(t.mode),
                    playtime_secs: saved,
                });
            }
        }
    }

    for id in ids {
        let track = {
            let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            map.get(&id).cloned()
        };
        let Some(mut t) = track else { continue };
        let now = chrono::Utc::now().timestamp_millis();

        // pid == 0 — автологин-трек уже обработан глобальной фазой выше
        if t.pid == 0 {
            continue;
        }

        if !winproc::is_alive(t.pid) {
            // Клиент закрылся. Roblox часто перезапускает сам себя при старте:
            // подхватываем новый процесс — самый старый из свежих, родившихся
            // не раньше нашего клиента (±1.5 с). Раньше требовали «ровно один
            // свежий процесс» — при перезапуске двух клиентов сразу оба трека
            // видели два процесса и не подцепляли ни один.
            let grace = t
                .dead_since
                .map(|s| now - s < 8_000)
                .unwrap_or(false);
            let young = now - t.started_at < 45_000 || grace;
            if young {
                let tracked: HashSet<u32> = {
                    let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                    map.values().map(|x| x.pid).collect()
                };
                let mut fresh: Vec<(u32, i64)> = winproc::roblox_pids()
                    .into_iter()
                    .filter(|p| !tracked.contains(p) && !taken_pids.contains(p) && *p != t.pid)
                    // только СВЕЖИЕ процессы: клиент перезапустил сам себя,
                    // чтобы не прихватить чужой давно работающий клиент
                    .filter_map(|p| {
                        winproc::process_start_time(p)
                            .filter(|st| now - *st < 20_000)
                            .map(|st| (p, st))
                    })
                    .collect();
                fresh.sort_by_key(|&(_, st)| st);
                let pick = fresh
                    .iter()
                    .copied()
                    .find(|(p, st)| *st >= t.started_at - 1500 && !taken_pids.contains(p));
                if let Some((new_pid, _)) = pick {
                    taken_pids.insert(new_pid);
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
                                mode: t.mode.clone(),
                                acc_secs: t.acc_secs,
                                last_tick_ms: now,
                                ticks: 0,
                                dead_since: None,
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
                // Процесс исчез, а новый ещё не появился — даём клиенту
                // до 8 с на самоперезапуск, не вынося вердикт «вышел».
                let since = t.dead_since.unwrap_or(now);
                if now - since < 8_000 {
                    if t.dead_since.is_none() {
                        t.dead_since = Some(now);
                        t.status = "launching".into();
                        t.last_title = None;
                    }
                    {
                        let mut map =
                            state.instances.lock().unwrap_or_else(|e| e.into_inner());
                        map.insert(id.clone(), t.clone());
                    }
                    changed = true;
                    snapshot.push(InstanceDto {
                        account_id: id.clone(),
                        pid: t.pid,
                        status: t.status.clone(),
                        started_at: t.started_at,
                        title: None,
                        mode: Some(t.mode.clone()),
                        playtime_secs: saved_playtime
                            .get(&id)
                            .copied()
                            .unwrap_or(0)
                            + t.acc_secs,
                    });
                    continue;
                }
                // 8 с прошло, клиент не вернулся — считаем выходом
            }

            // Реальный выход: различаем нормальное закрытие и краш по коду.
            let code = winproc::exit_code(t.pid);
            // Добираем время с последнего тика (только если был в игре)
            if t.status == "ingame" {
                let tail_ms = (now - t.last_tick_ms).clamp(0, 10_000);
                t.acc_secs += (tail_ms / 1000) as u64;
            }
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

        // Клиент жив: статус по заголовку окна.
        // window_title возвращает самый информативный заголовок видимого окна:
        // в меню это просто "Roblox", в игре — название игры (обычно
        // в формате "Игра - Roblox"). Если все окна — "Roblox", значит меню.
        let raw_title = winproc::window_title(t.pid);
        let status = match &raw_title {
            Some(s)
                if !s.trim().is_empty()
                    && !s.trim().eq_ignore_ascii_case("roblox") =>
            {
                "ingame"
            }
            _ => "online",
        };

        // Копим наработку ТОЛЬКО пока в игре (время в меню не считается),
        // дельта между тиками с потолком 10 с.
        if status == "ingame" {
            let delta_ms = (now - t.last_tick_ms).clamp(0, 10_000);
            t.acc_secs += (delta_ms / 1000) as u64;
        }
        t.last_tick_ms = now;
        t.ticks = t.ticks.saturating_add(1);
        if t.ticks >= 15 {
            // Раз в ~30 с сохраняем наработку на диск.
            t.ticks = 0;
            flush_playtime(state.inner());
        }

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
            mode: Some(t.mode),
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
