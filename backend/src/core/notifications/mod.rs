//! Structured operational notifications (log today; external channels later).

use tracing::{error, info, warn};

mod stub;

#[allow(unused_imports)]
pub use stub::StubChannelNotifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    #[allow(dead_code)]
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub severity: Severity,
    pub target: &'static str,
    pub message: String,
}

pub trait Notifier {
    fn notify(&self, event: &Notification);
}

/// Emits notifications through the tracing stack (stderr + rotated JSON logs).
#[derive(Debug, Default, Clone, Copy)]
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, event: &Notification) {
        let channel = event.target;
        let message = &event.message;
        match event.severity {
            Severity::Info => info!(target: "notifications", channel, "{message}"),
            Severity::Warning => warn!(target: "notifications", channel, "{message}"),
            Severity::Critical => error!(target: "notifications", channel, "{message}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct RecordingNotifier {
        events: Arc<Mutex<Vec<Notification>>>,
    }

    impl Notifier for RecordingNotifier {
        fn notify(&self, event: &Notification) {
            self.events.lock().expect("lock").push(event.clone());
        }
    }

    #[test]
    fn log_notifier_compiles_and_runs() {
        let notifier = LogNotifier;
        notifier.notify(&Notification {
            severity: Severity::Info,
            target: "notifications::test",
            message: "probe".into(),
        });
    }

    #[test]
    fn stub_channel_delegates_to_inner_notifier() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let inner = RecordingNotifier {
            events: Arc::clone(&events),
        };
        let notifier = StubChannelNotifier::new("slack", inner);
        notifier.notify(&Notification {
            severity: Severity::Warning,
            target: "notifications::persistence",
            message: "archive gap".into(),
        });
        let recorded = events.lock().expect("lock");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].message, "archive gap");
    }
}
