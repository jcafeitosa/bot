#![allow(unused_imports)]
//! Monitor application layer: startup, supervisor loop, persistence health, presentation contract.

#[allow(unused_imports)]
pub mod controllers;
pub mod models;
pub mod views;

#[allow(unused_imports)]
pub use controllers::handle::MonitorHandle;
pub use controllers::persistence_health::{PersistenceHealth, PersistenceState};
pub use controllers::startup::{
    bootstrap_monitor, connect_database, persistence_required, MonitorStore, StartupError,
};
pub use controllers::supervisor::run;
pub use views::presentation_contract::{
    MonitorCommand, MonitorEvent, MonitorNotice, MonitorSendError, MonitorSnapshot,
};
