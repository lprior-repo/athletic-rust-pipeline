use chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams;
use chromiumoxide::Page;
use serde::Deserialize;
use tokio::time::timeout;

use super::invocation::{self, Invocation};
use super::script::{ABORT_FUNCTION, FETCH_TIMEOUT};
use crate::gate::ProfileGate;
use crate::BrowserError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CleanupResult {
    Confirmed,
    Unconfirmed,
}

pub(super) async fn cleanup(
    page: &Page,
    gate: &ProfileGate,
    invocation: Option<&Invocation>,
) -> CleanupResult {
    if let Some(invocation) = invocation {
        if matches!(
            timeout(FETCH_TIMEOUT, confirm_abort(page, invocation)).await,
            Ok(Ok(()))
        ) {
            return CleanupResult::Confirmed;
        }
    }
    close_page_bounded(page, gate).await;
    CleanupResult::Unconfirmed
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AbortConfirmation {
    ok: bool,
    confirmed: bool,
}

async fn confirm_abort(page: &Page, invocation: &Invocation) -> Result<(), BrowserError> {
    let command = CallFunctionOnParams::builder()
        .function_declaration(ABORT_FUNCTION)
        .object_id(invocation.entry_id().clone())
        .return_by_value(true)
        .await_promise(true)
        .build()
        .map_err(|_| BrowserError::Protocol)?;
    let response = page
        .execute(command)
        .await
        .map_err(|_| BrowserError::Transport)?
        .result;
    if response.exception_details.is_some() {
        return Err(BrowserError::Protocol);
    }
    let confirmation: AbortConfirmation =
        serde_json::from_value(response.result.value.ok_or(BrowserError::Protocol)?)
            .map_err(|_| BrowserError::Protocol)?;
    if !confirmation.ok || !confirmation.confirmed {
        return Err(BrowserError::Protocol);
    }
    invocation::release(page, invocation).await
}

async fn close_page_bounded(page: &Page, gate: &ProfileGate) {
    gate.revoke();
    let close_fut = page.clone().close();
    tokio::pin!(close_fut);
    match timeout(FETCH_TIMEOUT, close_fut).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::error!(%error, "page close failed"),
        Err(_) => tracing::error!("page close timed out"),
    }
}
