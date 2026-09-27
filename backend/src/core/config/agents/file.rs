use super::super::load::env_override_string;
use super::super::system::SystemConfig;

pub fn monitor_agency_raw() -> Option<String> {
    let defaults = SystemConfig::active();
    let value = env_override_string("BOT_AGENCY", &defaults.agents.monitor_agency);
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn monitor_agency_raw_set() -> bool {
    monitor_agency_raw().is_some()
}
