use tracing::debug;

use super::{Notification, Notifier};

/// Placeholder for Slack, email, or paging integrations. Forwards to an inner notifier and
/// records that the external channel is not configured yet.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StubChannelNotifier<N> {
    channel: &'static str,
    inner: N,
}

impl<N> StubChannelNotifier<N> {
    #[allow(dead_code)]
    pub fn new(channel: &'static str, inner: N) -> Self {
        Self { channel, inner }
    }
}

impl<N: Notifier> Notifier for StubChannelNotifier<N> {
    fn notify(&self, event: &Notification) {
        debug!(
            target: "notifications::channel",
            channel = self.channel,
            severity = ?event.severity,
            "External notification channel not configured; delivering via inner notifier"
        );
        self.inner.notify(event);
    }
}
