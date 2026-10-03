use super::super::{BrowserState, BrowserStatus};
use crate::clock::Clock;
use std::sync::{Mutex, RwLock};
use tokio::time::Instant;

pub(super) fn usable_manager(state: BrowserState) -> bool {
    state != BrowserState::Stopped
}

pub(crate) fn read_status(status: &RwLock<BrowserStatus>) -> BrowserStatus {
    match status.read() {
        Ok(value) => value.clone(),
        Err(error) => error.into_inner().clone(),
    }
}

pub(crate) fn write_state(status: &RwLock<BrowserStatus>, state: BrowserState) {
    match status.write() {
        Ok(mut value) => value.state = state,
        Err(error) => error.into_inner().state = state,
    }
}

pub(crate) fn remaining_ms(clock: &dyn Clock, cooldown: &Mutex<Option<Instant>>) -> u64 {
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
        .map_or(u64::MAX, |value| value)
    })
}
