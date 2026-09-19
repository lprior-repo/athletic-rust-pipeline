use super::{
    browser::{BrowserState, BrowserStatus},
    Runtime,
};
use restate_sdk::prelude::*;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy)]
pub(crate) enum BrowserAction {
    Inspect,
    Recover,
    HumanRequired,
}

pub(crate) async fn act(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    action: BrowserAction,
) -> Result<BrowserStatus, HandlerError> {
    ctx.run(move || async move {
        let browser = runtime.ensure_browser().await.map_err(|_| {
            TerminalError::new(
                "persistent browser startup failed; inspect local worker diagnostics",
            )
        })?;
        let status = match action {
            BrowserAction::Inspect => browser.inspect().await,
            BrowserAction::Recover => browser.recover().await,
            BrowserAction::HumanRequired => {
                browser.mark_human_required();
                Ok(browser.status())
            }
        }
        .map_err(|error| TerminalError::new(error.to_string()))?;
        Ok::<_, HandlerError>(Json(status))
    })
    .name("browser profile lifecycle action")
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|value| value.0)
    .map_err(Into::into)
}

pub(crate) fn physical_status(runtime: &Runtime) -> BrowserStatus {
    runtime.browser().map_or_else(
        || BrowserStatus {
            state: BrowserState::Stopped,
            active_requests: 0,
            tabs: runtime.config.browser_settings().tabs,
            cooldown_ms: 0,
        },
        |browser| browser.status(),
    )
}

pub(crate) async fn now_ms(ctx: &ObjectContext<'_>) -> Result<u64, HandlerError> {
    ctx.run(|| async {
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
        Ok(u64::try_from(elapsed.as_millis())?)
    })
    .name("browser workflow clock")
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map_err(Into::into)
}
