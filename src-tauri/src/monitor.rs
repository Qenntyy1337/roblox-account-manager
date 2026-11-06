//! Фоновый монитор запущенных клиентов: жив ли процесс, заголовок окна
//! (в меню / в игре), события выхода. Публикует снапшот "instances".

use serde::Serialize;
use std::time::Duration;

use crate::commands::{log_event, AppState};
use crate::winproc;

#[derive(Debug, Clone)]
pub struct Track {
    pub pid: u32,
    pub status: String,
    pub started_at: i64,
    pub last_title: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct InstanceDto {
    pub account_id: String,
    pub pid: u32,
    pub status: String,
    pub started_at: i64,
    pub title: Option<String>,
}

pub fn snapshot(state: &AppState) -> Vec<InstanceDto> {
    let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
    map.iter()
        .map(|(id, t)| InstanceDto {
            account_id: id.clone(),
            pid: t.pid,
            status: t.status.clone(),
            started_at: t.started_at,
            title: t.last_title.clone(),
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

    let mut changed = false;
    let mut snapshot = Vec::new();

    for id in ids {
        let track = {
            let map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            map.get(&id).cloned()
        };
        let Some(mut t) = track else { continue };

        if !winproc::is_alive(t.pid) {
            // Клиент закрылся
            let username = {
                let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
                cfg.accounts
                    .iter()
                    .find(|a| a.id == id)
                    .map(|a| a.username.clone())
            };
            {
                let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
                map.remove(&id);
            }
            changed = true;
            log_event(
                app,
                "info",
                username.as_deref(),
                &format!("Client exited (pid {})", t.pid),
            );
            continue;
        }

        let title = winproc::window_title(t.pid);
        let status = match &title {
            Some(s)
                if !s.is_empty()
                    && s != "Roblox"
                    && s.to_ascii_lowercase().contains("roblox") =>
            {
                "ingame"
            }
            _ => "online",
        };

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
        }

        t.status = status.to_string();
        t.last_title = title;

        {
            let mut map = state.instances.lock().unwrap_or_else(|e| e.into_inner());
            map.insert(id.clone(), t.clone());
        }
        changed = true;

        snapshot.push(InstanceDto {
            account_id: id,
            pid: t.pid,
            status: t.status,
            started_at: t.started_at,
            title: t.last_title,
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
