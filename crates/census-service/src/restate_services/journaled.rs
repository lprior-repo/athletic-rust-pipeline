use std::sync::Arc;

use census_store::clock::Clock;
use restate_sdk::prelude::*;

pub(crate) async fn journaled_today(
    ctx: &ObjectContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    ctx.run(move || async move {
        let clock = Arc::clone(&clock);
        Ok::<_, restate_sdk::errors::HandlerError>(clock.today())
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map_err(restate_sdk::errors::HandlerError::from)
}

pub(crate) async fn journaled_today_workflow(
    ctx: &WorkflowContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    ctx.run(move || async move {
        let clock = Arc::clone(&clock);
        Ok::<_, restate_sdk::errors::HandlerError>(clock.today())
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map_err(restate_sdk::errors::HandlerError::from)
}
