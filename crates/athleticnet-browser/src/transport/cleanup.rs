use chromiumoxide::cdp::js_protocol::runtime;
use chromiumoxide::Page;
use tokio::time::timeout;

use super::script::{ABORT_FUNCTION, FETCH_TIMEOUT};
use crate::gate::ProfileGate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanupResult {
    Confirmed,
    Unconfirmed,
}

pub(crate) async fn cleanup(page: &Page, gate: &ProfileGate) -> CleanupResult {
    match confirm_abort(page).await {
        Some(true) => CleanupResult::Confirmed,
        _ => unconfirmed(page, gate).await,
    }
}

async fn confirm_abort(page: &Page) -> Option<bool> {
    let call = runtime::CallFunctionOnParams::builder()
        .function_declaration(ABORT_FUNCTION)
        .return_by_value(true)
        .await_promise(true)
        .build()
        .ok()?;
    let value = timeout(FETCH_TIMEOUT, page.evaluate_function(call))
        .await
        .ok()?
        .ok()?;
    value
        .into_value::<serde_json::Value>()
        .ok()
        .and_then(|v| v.get("confirmed").and_then(|c| c.as_bool()))
}

async fn unconfirmed(page: &Page, gate: &ProfileGate) -> CleanupResult {
    close_page_bounded(page, gate).await;
    CleanupResult::Unconfirmed
}

async fn close_page_bounded(page: &Page, gate: &ProfileGate) {
    gate.revoke();
    let close_fut = page.clone().close();
    tokio::pin!(close_fut);
    match timeout(FETCH_TIMEOUT, close_fut).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::error!("page close failed: {e}"),
        Err(_) => tracing::error!("page close timed out"),
    }
}
