#![allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorCommand {
    Pause,
    Resume,
    Refresh,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorNotice {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorEvent {
    StateChanged(MonitorSnapshot),
    Notice(MonitorNotice),
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorSendError {
    Full,
    Closed,
}
