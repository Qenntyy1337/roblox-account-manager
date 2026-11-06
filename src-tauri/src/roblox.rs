//! Взаимодействие с публичными API Roblox: проверка cookie, аватар,
//! presence-статусы. Все запросы с таймаутами и без паник.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

const USER_AGENT: &str = "RobloxAccountManager/1.0 (+tauri)";

pub struct RbxUser {
    pub id: u64,
    pub name: String,
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .user_agent(USER_AGENT)
            .build()
    })
}

fn cookie_header(cookie: &str) -> String {
    format!(".ROBLOSECURITY={}", cookie.trim())
}

/// Нормализация вставленного cookie: убираем пробелы/переносы по краям
/// и внутри, кавычки и префикс вида `.ROBLOSECURITY:`/`=` на случай,
/// если пользователь скопировал имя cookie вместе со значением.
pub fn normalize_cookie(input: &str) -> String {
    let s = input.trim();
    // снять кавычки по краям (если вставили вместе с ".ROBLOSECURITY:...")
    let s = s.trim_matches(|c| c == '"' || c == '\'');
    // снять префикс имени cookie (регистронезависимо; префикс — чистый ASCII)
    const PREFIXES: [&str; 2] = [".roblosecurity:", ".roblosecurity="];
    let lower = s.to_ascii_lowercase();
    let mut skip = 0usize;
    for p in PREFIXES {
        if lower.starts_with(p) {
            skip = p.len();
            break;
        }
    }
    let stripped = &s[skip..];
    // снять кавычки ещё раз (бывают внутри после префикса: .ROBLOSECURITY:"...")
    let stripped = stripped.trim_matches(|c| c == '"' || c == '\'');
    // удалить все пробельные символы (переносы строк при копировании)
    stripped.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Подробная ошибка проверки cookie — для диагностики в UI и логах.
#[derive(Debug, Clone)]
pub enum CookieCheckError {
    /// Roblox ответил 401/403 или не вернул пользователя: cookie невалиден/истёк.
    Invalid,
    /// 429: слишком много проверок подряд.
    RateLimited,
    /// Транспортная ошибка (таймаут, DNS, TLS, файрвол).
    Network(String),
}

impl CookieCheckError {
    /// Машинный ключ причины (для локализации во фронтенде).
    pub fn reason(&self) -> &'static str {
        match self {
            CookieCheckError::Invalid => "invalid",
            CookieCheckError::RateLimited => "rate_limited",
            CookieCheckError::Network(_) => "network",
        }
    }

    /// Техническая деталь для лога/подсказки (английский, как весь лог).
    pub fn detail(&self) -> String {
        match self {
            CookieCheckError::Invalid => {
                "Roblox rejected the cookie (401/403) — invalid or expired".into()
            }
            CookieCheckError::RateLimited => "HTTP 429 — too many validation attempts".into(),
            CookieCheckError::Network(e) => format!("network error: {e}"),
        }
    }
}

/// Проверить .ROBLOSECURITY с подробной ошибкой: вернёт id и имя пользователя,
/// если cookie валиден.
pub fn validate_cookie_ex(cookie: &str) -> Result<RbxUser, CookieCheckError> {
    let resp = agent()
        .get("https://users.roblox.com/v1/users/authenticated")
        .set("Cookie", &cookie_header(cookie))
        .set("Accept", "application/json")
        .call();
    match resp {
        Ok(r) => {
            let j: serde_json::Value = r
                .into_json()
                .map_err(|e| CookieCheckError::Network(e.to_string()))?;
            let id = j.get("id").and_then(|v| v.as_u64());
            let name = j
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match (id, name) {
                (Some(id), Some(name)) if !name.is_empty() => Ok(RbxUser { id, name }),
                _ => Err(CookieCheckError::Invalid),
            }
        }
        Err(ureq::Error::Status(code, _)) => {
            if code == 429 {
                Err(CookieCheckError::RateLimited)
            } else {
                Err(CookieCheckError::Invalid)
            }
        }
        Err(e) => Err(CookieCheckError::Network(e.to_string())),
    }
}

/// Проверить .ROBLOSECURITY: вернёт id и имя пользователя, если cookie валиден.
/// (Совместимая обёртка: нормализует ввод и сворачивает ошибку в Option.)
pub fn validate_cookie(cookie: &str) -> Option<RbxUser> {
    validate_cookie_ex(&normalize_cookie(cookie)).ok()
}

/// URL аватара (голова) пользователя.
pub fn avatar_url(user_id: u64) -> Option<String> {
    let url = format!(
        "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={user_id}&size=150x150&format=Png&isCircular=false"
    );
    let resp = agent()
        .get(&url)
        .set("Accept", "application/json")
        .call();
    match resp {
        Ok(r) => {
            let j: serde_json::Value = r.into_json().ok()?;
            let image = j
                .get("data")?
                .get(0)?
                .get("imageUrl")?
                .as_str()?
                .to_string();
            if image.is_empty() {
                None
            } else {
                Some(image)
            }
        }
        Err(_) => None,
    }
}

/// Получить одноразовый auth ticket для автологина.
/// POST https://auth.roblox.com/v1/authentication-ticket с обработкой
/// x-csrf-token (первый ответ 403 выдаёт токен, повтор — возвращает билет
/// в заголовке RBXAuthenticationTicket).
pub fn auth_ticket(cookie: &str) -> Result<String, String> {
    let hdr = cookie_header(cookie);
    let url = "https://auth.roblox.com/v1/authentication-ticket";
    let body = serde_json::json!({});

    let build = || {
        agent()
            .post(url)
            .set("Cookie", &hdr)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
    };

    let resp = match build().send_json(body.clone()) {
        Ok(r) => Ok(r),
        Err(ureq::Error::Status(403, r)) => {
            let csrf = r
                .header("x-csrf-token")
                .ok_or_else(|| "403 без заголовка x-csrf-token".to_string())?
                .to_string();
            build()
                .set("x-csrf-token", &csrf)
                .send_json(body)
                .map_err(|e| match e {
                    ureq::Error::Status(code, _) => format!("HTTP {code}"),
                    other => other.to_string(),
                })
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("HTTP {code}")),
        Err(e) => Err(e.to_string()),
    }?;

    let ticket = resp
        .header("RBXAuthenticationTicket")
        .ok_or_else(|| "ответ без заголовка RBXAuthenticationTicket".to_string())?
        .to_string();
    if ticket.is_empty() {
        return Err("пустой auth ticket".into());
    }
    Ok(ticket)
}

/// URL протокола roblox-player: для автологина. gameinfo — base64 auth
/// ticket; клиент откроется уже залогиненным в нужный аккаунт.
pub fn player_protocol_url(ticket: &str) -> String {
    let gameinfo = B64.encode(ticket.as_bytes());
    format!(
        "roblox-player:1+launchtime:{}+gameinfo:{}",
        chrono::Utc::now().timestamp_millis(),
        gameinfo
    )
}

/// Presence-статусы: 0 — офлайн, 1 — на сайте, 2 — в игре, 3 — Studio.
/// POST с обработкой x-csrf-token (Roblox требует его для POST-запросов).
pub fn fetch_presence(cookie: &str, user_ids: &[u64]) -> Option<HashMap<u64, u8>> {
    if user_ids.is_empty() {
        return Some(HashMap::new());
    }
    let hdr = cookie_header(cookie);
    let body = serde_json::json!({ "userIds": user_ids });
    let url = "https://presence.roblox.com/v1/presence/users";

    let build = || {
        agent()
            .post(url)
            .set("Cookie", &hdr)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
    };

    let resp = match build().send_json(body.clone()) {
        Ok(r) => Some(r),
        Err(ureq::Error::Status(code, r)) if code == 403 => {
            let csrf = r.header("x-csrf-token")?.to_string();
            build().set("x-csrf-token", &csrf).send_json(body).ok()
        }
        Err(_) => None,
    }?;

    let j: serde_json::Value = resp.into_json().ok()?;
    let mut map = HashMap::new();
    if let Some(list) = j.get("userPresences").and_then(|v| v.as_array()) {
        for item in list {
            let uid = item.get("userId").and_then(|v| v.as_u64());
            let typ = item.get("type").and_then(|v| v.as_u64()).unwrap_or(0);
            if let Some(uid) = uid {
                map.insert(uid, typ.min(3) as u8);
            }
        }
    }
    Some(map)
}
