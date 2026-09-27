use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::models::{
    BotPromotionRecord, BotPromotionState, BotRuntimeStatus, BotsError, PromoteBotRequest,
};

pub trait BotRuntimePort: Send + Sync {
    fn status(&self) -> BotRuntimeStatus;
    fn promote(&self, request: PromoteBotRequest) -> Result<BotPromotionRecord, BotsError>;
    fn demote(&self) -> Result<(), BotsError>;
}

#[derive(Debug, Default)]
pub struct FailClosedBotRuntime;

impl BotRuntimePort for FailClosedBotRuntime {
    fn status(&self) -> BotRuntimeStatus {
        BotRuntimeStatus {
            runtime_enabled: false,
            active: None,
        }
    }

    fn promote(&self, _request: PromoteBotRequest) -> Result<BotPromotionRecord, BotsError> {
        Err(BotsError::RuntimeDisabled)
    }

    fn demote(&self) -> Result<(), BotsError> {
        Err(BotsError::RuntimeDisabled)
    }
}

#[derive(Debug, Default)]
pub struct InMemoryBotRuntime {
    active: Mutex<Option<BotPromotionRecord>>,
}

impl InMemoryBotRuntime {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl BotRuntimePort for InMemoryBotRuntime {
    fn status(&self) -> BotRuntimeStatus {
        let active = self.active.lock().expect("bot runtime lock").clone();
        BotRuntimeStatus {
            runtime_enabled: true,
            active,
        }
    }

    fn promote(&self, request: PromoteBotRequest) -> Result<BotPromotionRecord, BotsError> {
        request.validate()?;
        let record = BotPromotionRecord {
            bot_id: request.bot_id.trim().to_string(),
            promoted_by: request.promoted_by.trim().to_string(),
            promoted_at_unix_ms: now_unix_ms(),
            state: BotPromotionState::Active,
        };
        *self.active.lock().expect("bot runtime lock") = Some(record.clone());
        Ok(record)
    }

    fn demote(&self) -> Result<(), BotsError> {
        let mut guard = self.active.lock().expect("bot runtime lock");
        if guard.is_none() {
            return Err(BotsError::RuntimeNotPromoted);
        }
        *guard = None;
        Ok(())
    }
}

pub fn bot_runtime_from_env() -> std::sync::Arc<dyn BotRuntimePort> {
    match std::env::var("BOT_RUNTIME_ENABLED") {
        Ok(raw) if raw.trim().eq_ignore_ascii_case("true") => {
            std::sync::Arc::new(InMemoryBotRuntime::new())
        }
        Ok(raw) if !raw.trim().is_empty() && !raw.trim().eq_ignore_ascii_case("false") => {
            tracing::warn!(
                target: "api",
                value = raw.trim(),
                "unknown BOT_RUNTIME_ENABLED; using fail-closed bot runtime"
            );
            std::sync::Arc::new(FailClosedBotRuntime)
        }
        _ => std::sync::Arc::new(FailClosedBotRuntime),
    }
}

/// Process-wide bot runtime (HTTP `serve` and future monitor hooks share promotion state).
pub fn shared_bot_runtime() -> Arc<dyn BotRuntimePort> {
    static RUNTIME: OnceLock<Arc<dyn BotRuntimePort>> = OnceLock::new();
    RUNTIME.get_or_init(bot_runtime_from_env).clone()
}

/// Copies active bot promotion into the monitor snapshot (read-only seam for HTTP/TUI).
pub fn apply_bot_runtime_to_monitor_snapshot(
    snapshot: &mut crate::modules::monitor::views::presentation_contract::MonitorSnapshot,
    runtime: &dyn BotRuntimePort,
) {
    let status = runtime.status();
    snapshot.bot_runtime_enabled = status.runtime_enabled;
    snapshot.promoted_bot_id = status.active.as_ref().map(|r| r.bot_id.clone());
    snapshot.promoted_by = status.active.as_ref().map(|r| r.promoted_by.clone());
}

pub fn enrich_monitor_snapshot_from_shared_runtime(
    snapshot: &mut crate::modules::monitor::views::presentation_contract::MonitorSnapshot,
) {
    apply_bot_runtime_to_monitor_snapshot(snapshot, shared_bot_runtime().as_ref());
}

#[cfg(test)]
mod shared_runtime_tests {
    use super::*;

    #[test]
    fn shared_bot_runtime_returns_same_arc() {
        let a = shared_bot_runtime();
        let b = shared_bot_runtime();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn apply_bot_runtime_to_monitor_snapshot_copies_promotion() {
        use crate::modules::monitor::views::presentation_contract::MonitorSnapshot;
        let runtime = InMemoryBotRuntime::new();
        runtime
            .promote(PromoteBotRequest {
                bot_id: "sma-cross@1:5m:BTCUSDT".into(),
                promoted_by: "owner-1".into(),
            })
            .expect("promote");
        let mut snapshot = MonitorSnapshot::initial();
        apply_bot_runtime_to_monitor_snapshot(&mut snapshot, &runtime);
        assert!(snapshot.bot_runtime_enabled);
        assert_eq!(
            snapshot.promoted_bot_id.as_deref(),
            Some("sma-cross@1:5m:BTCUSDT")
        );
    }
}
