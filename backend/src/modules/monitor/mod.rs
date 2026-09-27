//! Monitor application layer: startup, supervisor loop, persistence health, presentation contract.

#![allow(unused_imports)] // public re-exports for presentation and integration tests

pub mod controllers;
pub mod models;
pub mod views;

#[allow(unused_imports)]
pub mod presentation_contract {
    pub use super::views::presentation_contract::*;
}

pub use controllers::handle::{MonitorHandle, SnapshotPublishError};
pub use controllers::persistence_health::{PersistenceHealth, PersistenceState};
pub use controllers::startup::{
    bootstrap_monitor, connect_database, persistence_required, MonitorStore, StartupError,
};
pub use controllers::supervisor::{run, run_with_agent_hook, spawn_headless_for_api};
pub use views::presentation_contract::{
    validate_bounded_label, validate_snapshot, Decimal, DecimalError, Environment,
    LabelValidationError, MonitorCommand, MonitorEvent, MonitorLogEntry, MonitorMarketSnapshot,
    MonitorNotice, MonitorRunState, MonitorSendError, MonitorSignal, MonitorSnapshot,
    NoticeSeverity, PersistenceStatus, RunMode, SnapshotValidationError,
};
pub use views::{
    AppEvent, Dashboard, DashboardHeader, MonitorSignalLabel, MonitorState, TerminalMarketRow,
    UiCommand,
};
