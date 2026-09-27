//! Shared TOML + env override helpers (TOML defaults committed, env for secrets/overrides).

use std::{env, fs, path::Path};

use serde::de::DeserializeOwned;

use crate::core::error::{BotError, BotResult};

/// Read `bot.toml` from an explicit path. Missing paths and directories are errors (no bundled fallback).
pub fn read_bot_config_file(path: &Path) -> BotResult<String> {
    fs::read_to_string(path)
        .map_err(|e| BotError::Configuration(format!("cannot read config {}: {e}", path.display())))
}

pub fn parse_embedded_toml<T: DeserializeOwned>(embedded: &str, label: &str) -> T {
    toml::from_str(embedded).unwrap_or_else(|e| {
        panic!("embedded {label} TOML is invalid: {e}");
    })
}

pub fn env_nonempty(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn env_override_bool(name: &'static str, toml_default: bool) -> bool {
    match env::var(name) {
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" | "" => false,
            _ => toml_default,
        },
        Err(_) => toml_default,
    }
}

pub fn env_override_string(name: &'static str, toml_default: &str) -> String {
    env_nonempty(name).unwrap_or_else(|| toml_default.to_string())
}

#[allow(dead_code)]
pub fn env_override_option_string(name: &'static str) -> Option<String> {
    env_nonempty(name)
}

#[allow(dead_code)]
pub fn require_env_nonempty(name: &'static str) -> BotResult<String> {
    env_nonempty(name).ok_or_else(|| {
        BotError::Configuration(format!("missing or empty environment variable {name}"))
    })
}
