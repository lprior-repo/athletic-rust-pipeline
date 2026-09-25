//! The service's date, read through the journal rather than from the clock.
//!
//! The clock is a trait so replay *can* be deterministic, but calling it directly is not: the date
//! lands in durable state (`ctx.set`) and in `run` results that Restate compares when an entry is
//! written again, so a replay that crosses midnight would fail the invocation with a journal
//! mismatch instead of replaying it. Journaling the read makes the date the journal's.
//!
//! One body per context type: the SDK's `run` is a trait method whose closure type does not survive
//! being wrapped in a generic, so the object and workflow forms cannot share an inner call. It lives
//! outside `mod.rs` for the §38 line budget, and is re-exported at the module root so every handler
//! keeps calling it at the path it always did.

use std::sync::Arc;

use census_store::clock::Clock;
use restate_sdk::prelude::*;

/// The service's date, read through the object context's journal.
pub(crate) async fn journaled_today(
    ctx: &ObjectContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    ctx.run(move || async move {
        let clock = Arc::clone(&clock);
        Ok::<_, restate_sdk::errors::HandlerError>(clock.today())
    })
    .await
    .map_err(restate_sdk::errors::HandlerError::from)
}

/// [`journaled_today`] for workflow handlers.
pub(crate) async fn journaled_today_workflow(
    ctx: &WorkflowContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    ctx.run(move || async move {
        let clock = Arc::clone(&clock);
        Ok::<_, restate_sdk::errors::HandlerError>(clock.today())
    })
    .await
    .map_err(restate_sdk::errors::HandlerError::from)
}
