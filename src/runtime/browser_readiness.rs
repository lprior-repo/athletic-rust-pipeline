use super::{
    browser::{BrowserError, BrowserManager, BrowserState, BrowserStatus},
    clock::{self, Clock},
    Runtime,
};
use restate_sdk::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(crate) enum BrowserAction {
    Inspect,
    Recover,
    Restart,
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
        let status = dispatch_action(&browser, action)
            .await
            .map_err(|error| TerminalError::new(error.to_string()))?;
        Ok::<_, HandlerError>(Json(status))
    })
    .name("browser profile lifecycle action")
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|value| value.0)
    .map_err(Into::into)
}

/// One operator action against a manager that is already known to be reachable.
async fn dispatch_action(
    browser: &BrowserManager,
    action: BrowserAction,
) -> Result<BrowserStatus, BrowserError> {
    match action {
        BrowserAction::Inspect => browser.inspect().await,
        BrowserAction::Recover => browser.recover().await,
        BrowserAction::Restart => browser.restart().await,
        BrowserAction::HumanRequired => {
            browser.mark_human_required();
            Ok(browser.status())
        }
    }
}

pub(crate) async fn physical_status(runtime: &Runtime) -> BrowserStatus {
    match runtime.browser().await {
        Some(browser) => browser.status(),
        None => BrowserStatus {
            state: BrowserState::Stopped,
            active_requests: 0,
            tabs: runtime.config.browser_settings().tabs,
            cooldown_ms: 0,
        },
    }
}

/// Journaled wall-clock read for the readiness workflow.
///
/// Thin naming of [`clock::journal_unix_ms`]: the name below is the journal identity of this
/// entry and MUST NOT change while an invocation that wrote it can still be replayed.
pub(crate) async fn now_ms(
    ctx: &ObjectContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<u64, HandlerError> {
    clock::journal_unix_ms(ctx, clock, "browser workflow clock").await
}
