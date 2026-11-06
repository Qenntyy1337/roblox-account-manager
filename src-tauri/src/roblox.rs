//! Взаимодействие с публичными API Roblox: проверка cookie, аватар,
//! presence-статусы. Все запросы с таймаутами и без паник.

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

/// Проверить .ROBLOSECURITY: вернёт id и имя пользователя, если cookie валиден.
pub fn validate_cookie(cookie: &str) -> Option<RbxUser> {
    let resp = agent()
        .get("https://users.roblox.com/v1/users/authenticated")
        .set("Cookie", &cookie_header(cookie))
        .set("Accept", "application/json")
        .call();
    match resp {
        Ok(r) => {
            let j: serde_json::Value = r.into_json().ok()?;
            let id = j.get("id")?.as_u64()?;
            let name = j.get("name")?.as_str()?.to_string();
            if name.is_empty() {
                None
            } else {
                Some(RbxUser { id, name })
            }
        }
        Err(_) => None,
    }
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
