use super::super::load::env_override_bool;
use super::super::system::SystemConfig;

pub fn bot_runtime_enabled_from_env() -> bool {
    let defaults = SystemConfig::active();
    env_override_bool("BOT_RUNTIME_ENABLED", defaults.bots.runtime_enabled)
}
