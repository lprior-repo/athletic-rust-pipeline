//! Shared status-snapshot accessors.
//!
//! The manager and the actor's submodules both read and write the one status snapshot, so these
//! accessors stay on the `lifecycle` path: `browser::state` reaches them as
//! `lifecycle::{read_status, remaining_ms, write_state}`. Every accessor recovers the poisoned
//! inner value, because a panic in one holder must not turn every later status read into a second
//! panic.

use super::super::{BrowserState, BrowserStatus};
use crate::runtime::clock::Clock;
use std::sync::{Mutex, RwLock};
use tokio::time::Instant;

/// A manager reporting a terminal `Stopped` status can never serve another
/// command, so the runtime rebuilds it rather than reusing a dead handle.
pub(super) fn usable_manager(state: BrowserState) -> bool {
    state != BrowserState::Stopped
}

pub(in crate::runtime::browser) fn read_status(status: &RwLock<BrowserStatus>) -> BrowserStatus {
    match status.read() {
        Ok(value) => value.clone(),
        Err(error) => error.into_inner().clone(),
    }
}

pub(in crate::runtime::browser) fn write_state(
    status: &RwLock<BrowserStatus>,
    state: BrowserState,
) {
    match status.write() {
        Ok(mut value) => value.state = state,
        Err(error) => error.into_inner().state = state,
    }
}

pub(in crate::runtime::browser) fn remaining_ms(
    clock: &dyn Clock,
    cooldown: &Mutex<Option<Instant>>,
) -> u64 {
    let until = match cooldown.lock() {
        Ok(value) => *value,
        Err(error) => *error.into_inner(),
    };
    until.map_or(0, |value| {
        u64::try_from(
            value
                .saturating_duration_since(clock.now_instant())
                .as_millis(),
        )
        .unwrap_or(u64::MAX)
    })
}
