//! Локальное хранение конфигурации: настройки, аккаунты (с зашифрованными
//! cookie). Атомарная запись через временный файл + rename.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub user_id: Option<u64>,
    pub avatar_url: Option<String>,
    pub cookie_ok: bool,
    pub note: String,
    pub created_at: i64,
    /// Суммарная наработка за все сессии, секунды.
    #[serde(default)]
    pub playtime_secs: u64,
    /// Последняя активность (конец последней сессии), unix ms.
    #[serde(default)]
    pub last_played: Option<i64>,
    /// Зашифрованный DPAPI .ROBLOSECURITY (base64). Не сериализуется наружу.
    #[serde(default, skip_serializing)]
    pub cookie_enc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub accent: String,
    pub language: String,
    pub bg_image: Option<String>,
    pub bg_opacity: i64,
    pub bg_blur: i64,
    pub launch_delay_ms: i64,
    pub isolate_data: bool,
    pub close_singleton: bool,
    /// Автологин: клиент запускается сразу с нужным аккаунтом (auth ticket).
    pub auto_login: bool,
    /// Звуковые эффекты интерфейса (клики, ошибки, краши).
    pub sound_enabled: bool,
    /// Уведомления Windows (краш, запуск, вход в игру).
    pub win_notifications: bool,
    pub roblox_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            accent: "#8b5cf6".into(),
            language: "ru".into(),
            bg_image: None,
            bg_opacity: 55,
            bg_blur: 14,
            launch_delay_ms: 900,
            isolate_data: true,
            close_singleton: true,
            auto_login: true,
            sound_enabled: true,
            win_notifications: true,
            roblox_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub settings: Settings,
    pub accounts: Vec<Account>,
}

pub fn load_or_default(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<Config>(&raw) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("[RAM] config.json повреждён ({e}); используются настройки по умолчанию");
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}

pub fn save(path: &Path, cfg: &Config) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}
