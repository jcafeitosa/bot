use std::env;

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
