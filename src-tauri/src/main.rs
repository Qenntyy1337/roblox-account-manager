//! Точка входа Roblox Account Manager (Tauri 2).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod monitor;
mod roblox;
mod store;
mod winproc;

use std::sync::Mutex;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("не удалось определить каталог данных приложения");
            let _ = std::fs::create_dir_all(&data_dir);
            let config_path = data_dir.join("config.json");
            let config = store::load_or_default(&config_path);

            app.manage(commands::AppState {
                data_dir,
                config_path,
                config: Mutex::new(config),
                instances: Mutex::new(std::collections::HashMap::new()),
                launch_lock: tauri::async_runtime::Mutex::new(()),
            });

            // Acrylic-подложка окна в Windows (тихо игнорируется при неудаче)
            #[cfg(windows)]
            if let Some(win) = app.get_webview_window("main") {
                let _ = window_vibrancy::apply_acrylic(&win, Some((12, 13, 20, 125)));
            }

            monitor::spawn(app.handle().clone());

            // Перехват имени ROBLOX_singletonEvent при старте менеджера:
            // пока менеджер жив, ни один клиент Roblox не сможет создать
            // реальный Event и «сигналить» другим клиентам. Если сейчас
            // работают клиенты — перехват случится позже, при запуске.
            let _ = winproc::squat_singleton_event();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::save_settings,
            commands::add_account,
            commands::update_account,
            commands::delete_account,
            commands::launch_account,
            commands::terminate_instance,
            commands::close_all,
            commands::validate_cookie,
            commands::detect_roblox_path,
            commands::pick_file,
            commands::export_logs,
            commands::open_path,
            commands::get_presence,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска приложения");
}
