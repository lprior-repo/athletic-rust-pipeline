//! The Restate-journaled wall-clock read.
//!
//! The [`Clock`] seam itself lives in `athleticnet-browser`: the transport needs it and cannot
//! depend on Restate. What stays here is the journaled read, because it needs an object context —
//! and journaling is a property of the invocation, not of the browser.

use athleticnet_browser::clock::Clock;
use restate_sdk::prelude::*;
use std::sync::Arc;

/// Read wall-clock time through the Restate journal, so a replay returns the original reading.
///
/// `name` is the journal identity of the `ctx.run` entry. Two reads inside one invocation MUST use
/// distinct names, and a name already written into a deployed journal MUST NOT be renamed: the VM
/// matches entries by name, and a rename fails replay with a non-determinism error. `""` is the SDK
/// default and preserves an unnamed entry.
pub(crate) async fn journal_unix_ms(
    ctx: &ObjectContext<'_>,
    clock: &Arc<dyn Clock>,
    name: &'static str,
) -> Result<u64, HandlerError> {
    let clock = clock.clone();
    ctx.run(move || async move { clock.now_unix_ms().map_err(HandlerError::from) })
        .name(name)
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await
        .map_err(Into::into)
}
