#![allow(dead_code)]
//! Public handle seam; snapshot publisher wiring follows in supervisor.

use tokio::sync::{broadcast, mpsc, watch};
use tokio_util::sync::CancellationToken;

use crate::modules::monitor::views::presentation_contract::{
    validate_snapshot, MonitorCommand, MonitorEvent, MonitorSendError, MonitorSnapshot,
    SnapshotValidationError,
};

#[derive(Clone)]
pub struct MonitorHandle {
    commands: mpsc::Sender<MonitorCommand>,
    events: broadcast::Sender<MonitorEvent>,
    snapshots: watch::Sender<MonitorSnapshot>,
    cancellation: CancellationToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotPublishError {
    Validation(SnapshotValidationError),
    SnapshotClosed,
    EventClosed,
}

impl MonitorHandle {
    pub fn channel(
        command_capacity: usize,
        event_capacity: usize,
    ) -> (
        Self,
        mpsc::Receiver<MonitorCommand>,
        broadcast::Receiver<MonitorEvent>,
    ) {
        let (commands, command_rx) = mpsc::channel(command_capacity);
        let (events, event_rx) = broadcast::channel(event_capacity);
        let (snapshots, _) = watch::channel(MonitorSnapshot::initial());
        let handle = Self {
            commands,
            events,
            snapshots,
            cancellation: CancellationToken::new(),
        };
        (handle, command_rx, event_rx)
    }

    pub fn send(&self, command: MonitorCommand) -> Result<(), MonitorSendError> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => MonitorSendError::Full,
                mpsc::error::TrySendError::Closed(_) => MonitorSendError::Closed,
            })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MonitorEvent> {
        self.events.subscribe()
    }

    pub fn latest_snapshot(&self) -> watch::Receiver<MonitorSnapshot> {
        self.snapshots.subscribe()
    }

    /// Publishes the authoritative snapshot, then emits a wake-up event (watch before broadcast).
    pub fn publish_snapshot(&self, snapshot: MonitorSnapshot) -> Result<(), SnapshotPublishError> {
        validate_snapshot(&snapshot).map_err(SnapshotPublishError::Validation)?;
        let revision = snapshot.revision;
        self.snapshots
            .send(snapshot)
            .map_err(|_| SnapshotPublishError::SnapshotClosed)?;
        self.events
            .send(MonitorEvent::StateChanged { revision })
            .map_err(|_| SnapshotPublishError::EventClosed)?;
        Ok(())
    }

    pub fn publish_event(
        &self,
        event: MonitorEvent,
    ) -> Result<usize, broadcast::error::SendError<MonitorEvent>> {
        self.events.send(event)
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    use crate::modules::monitor::views::presentation_contract::MonitorCommand;

    #[tokio::test]
    async fn send_delivers_command_and_fails_when_receiver_is_closed() {
        let (handle, mut commands, _) = MonitorHandle::channel(4, 4);
        handle.send(MonitorCommand::Pause).unwrap();
        assert!(matches!(commands.recv().await, Some(MonitorCommand::Pause)));
        drop(commands);
        assert_eq!(
            handle.send(MonitorCommand::Resume),
            Err(MonitorSendError::Closed)
        );
    }

    #[tokio::test]
    async fn send_returns_full_when_command_queue_is_saturated() {
        let (handle, _commands, _) = MonitorHandle::channel(1, 4);
        handle.send(MonitorCommand::Pause).unwrap();
        assert_eq!(
            handle.send(MonitorCommand::Resume),
            Err(MonitorSendError::Full)
        );
    }

    #[tokio::test]
    async fn subscribe_receives_events_and_exposes_lag_and_disconnect() {
        let (handle, _commands, _) = MonitorHandle::channel(4, 1);
        let mut events = handle.subscribe();
        handle.publish_event(MonitorEvent::Stopped).unwrap();
        assert!(matches!(events.recv().await, Ok(MonitorEvent::Stopped)));
        handle.publish_event(MonitorEvent::Stopped).unwrap();
        handle.publish_event(MonitorEvent::Stopped).unwrap();
        assert!(matches!(
            events.recv().await,
            Err(broadcast::error::RecvError::Lagged(1))
        ));
        assert!(matches!(events.recv().await, Ok(MonitorEvent::Stopped)));
        drop(handle);
        assert!(matches!(
            events.recv().await,
            Err(broadcast::error::RecvError::Closed)
        ));
    }

    #[tokio::test]
    async fn publish_snapshot_updates_watch_before_state_changed() {
        let (handle, _commands, _) = MonitorHandle::channel(4, 8);
        let mut snapshots = handle.latest_snapshot();
        let mut events = handle.subscribe();
        let mut next = MonitorSnapshot::initial();
        next.revision = 1;
        next.symbol = "BTC/USDT".into();
        handle.publish_snapshot(next.clone()).unwrap();
        assert_eq!(*snapshots.borrow_and_update(), next);
        assert!(matches!(
            events.recv().await,
            Ok(MonitorEvent::StateChanged { revision: 1 })
        ));
    }
}
