#![allow(dead_code)]
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::modules::monitor::views::presentation_contract::{
    MonitorCommand, MonitorEvent, MonitorSendError,
};

#[derive(Clone)]
pub struct MonitorHandle {
    commands: mpsc::Sender<MonitorCommand>,
    events: broadcast::Sender<MonitorEvent>,
    cancellation: CancellationToken,
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
        let handle = Self {
            commands,
            events,
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

    pub fn publish(
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
    async fn subscribe_receives_events_and_exposes_lag_and_disconnect() {
        let (handle, _commands, _) = MonitorHandle::channel(4, 1);
        let mut events = handle.subscribe();
        handle.publish(MonitorEvent::Stopped).unwrap();
        assert!(matches!(events.recv().await, Ok(MonitorEvent::Stopped)));
        handle.publish(MonitorEvent::Stopped).unwrap();
        handle.publish(MonitorEvent::Stopped).unwrap();
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
}
