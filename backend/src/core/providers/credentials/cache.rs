use std::collections::HashMap;
use std::sync::Mutex;

use sqlx::PgPool;

pub const PROVIDER_TYPESAFE: &str = "typesafe";
pub const PROVIDER_OPENAI: &str = "openai";
pub const PROVIDER_NVIDIA: &str = "nvidia";
pub const PROVIDER_NGC: &str = "ngc";
pub const KEY_API_KEY: &str = "api_key";

static CACHE: Mutex<Option<HashMap<(String, String), String>>> = Mutex::new(None);

pub async fn reload_from_pool(pool: &PgPool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT provider_id, key_name, secret FROM provider_credentials",
    )
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for (provider_id, key_name, secret) in rows {
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            continue;
        }
        map.insert((provider_id, key_name), trimmed.to_string());
    }
    *CACHE.lock().expect("provider credential cache lock") = Some(map);
    Ok(())
}

fn cache_get(provider_id: &str, key_name: &str) -> Option<String> {
    let guard = CACHE.lock().expect("provider credential cache lock");
    let map = guard.as_ref()?;
    map.get(&(provider_id.to_string(), key_name.to_string()))
        .cloned()
}

/// Primary: PostgreSQL cache. Fallback: deprecated bootstrap env vars (dev/tests only).
pub fn lookup_secret(
    provider_id: &str,
    key_name: &str,
    env_fallback: &[&'static str],
) -> Option<String> {
    if let Some(value) = cache_get(provider_id, key_name) {
        return Some(value);
    }
    for name in env_fallback {
        if let Ok(raw) = std::env::var(name) {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                tracing::debug!(
                    target: "providers",
                    env = name,
                    "using deprecated env bootstrap for provider credential (prefer PostgreSQL provider_credentials)"
                );
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn env_fallback_when_cache_empty() {
        with_env_test_lock(|| {
            *CACHE.lock().expect("lock") = None;
            std::env::set_var("TYPESAFE_API_KEY", "dev-key");
            let value = lookup_secret(PROVIDER_TYPESAFE, KEY_API_KEY, &["TYPESAFE_API_KEY"]);
            std::env::remove_var("TYPESAFE_API_KEY");
            assert_eq!(value.as_deref(), Some("dev-key"));
        });
    }
}
