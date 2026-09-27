//! Serializes `std::env` mutations across parallel unit tests in one process.

use std::sync::{Mutex, MutexGuard};

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Hold for the whole test (including `.await`) while reading or mutating process env.
pub struct EnvTestGuard {
    _guard: MutexGuard<'static, ()>,
}

impl EnvTestGuard {
    pub fn acquire() -> Self {
        Self { _guard: lock_env() }
    }
}

pub fn with_env_test_lock<F: FnOnce()>(f: F) {
    let _guard = lock_env();
    f();
}
