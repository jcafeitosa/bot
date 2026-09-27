use std::env;

#[allow(dead_code)]
pub fn optional_trimmed(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn parse_bool_flag(name: &'static str, default: bool) -> Result<bool, String> {
    match env::var(name) {
        Ok(raw) => {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => Ok(true),
                "0" | "false" | "no" | "off" | "" => Ok(false),
                other => Err(format!("invalid {name} value: {other}")),
            }
        }
        Err(_) => Ok(default),
    }
}

#[allow(dead_code)]
pub fn optional_var_raw(name: &'static str) -> Result<Option<String>, env::VarError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(other) => Err(other),
    }
}
