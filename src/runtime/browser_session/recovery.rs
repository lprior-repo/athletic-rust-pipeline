//! Re-arming an exhausted recovery latch for an explicit operator request.
//!
//! `await_ready` with `operator = true` clears the challenge bookkeeping and, when the session is
//! latched in `HumanRequired`, asks the browser for one `Restart`. Internal waiters never reach
//! here, so a stall only clears when an operator re-runs readiness.

use super::super::browser_readiness::{self, BrowserAction};
use super::super::Runtime;
use athleticnet_browser::BrowserState;
use restate_sdk::prelude::*;
use std::sync::Arc;

/// Re-arm an exhausted session for one bounded attempt when an operator
/// explicitly re-runs readiness. Without this, every caller keeps observing
/// the same stalled state while the recovery latch stays consumed.
pub(super) async fn release_exhausted_recovery(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
) -> Result<(), HandlerError> {
    ctx.clear("challenge-started-ms");
    ctx.clear("recovery-issued");
    let physical = browser_readiness::physical_status(&runtime).await;
    if physical.state == BrowserState::HumanRequired {
        browser_readiness::act(ctx, runtime, BrowserAction::Restart).await?;
    }
    Ok(())
}
