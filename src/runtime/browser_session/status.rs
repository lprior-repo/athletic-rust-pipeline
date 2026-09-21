//! Durable-over-physical status projection and the session key guard.

use super::super::browser::{BrowserState, BrowserStatus};
use super::BROWSER_SESSION_KEY;
use restate_sdk::prelude::*;

/// Merge the durable status over the physical one.
///
/// The durable record wins whenever the physical browser reports `Ready` but the journal says
/// otherwise: a previous invocation decided this session is not ready to serve, and a fresh
/// bootstrap must not silently undo that verdict.
pub(super) fn merge_status(
    durable: Option<BrowserStatus>,
    mut physical: BrowserStatus,
) -> BrowserStatus {
    if let Some(durable) = durable {
        if physical.state == BrowserState::Ready && durable.state != BrowserState::Ready {
            physical.state = durable.state;
            physical.cooldown_ms = durable.cooldown_ms;
        }
    }
    physical
}

pub(super) fn validate_key(key: &str) -> Result<(), HandlerError> {
    if key != BROWSER_SESSION_KEY {
        return Err(TerminalError::new("invalid browser session key").into());
    }
    Ok(())
}
