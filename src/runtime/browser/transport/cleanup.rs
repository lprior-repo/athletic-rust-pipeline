use chromiumoxide::cdp::js_protocol::runtime;
use chromiumoxide::Page;
use tokio::time::timeout;

use super::script::{ABORT_FUNCTION, FETCH_TIMEOUT};
use crate::runtime::browser::gate::ProfileGate;

/// Cleanup confirmation from ABORT's awaited completion promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanupResult {
    /// ABORT confirmed — fetch settled cleanly.
    Confirmed,
    /// ABORT did NOT confirm — page may be compromised.
    Unconfirmed,
}

/// Owned cleanup: abort the JS fetch and confirm completion via a real promise.
///
/// Script writes a completion promise in FETCH finally that ABORT awaits.
/// Rust calls ABORT with await_promise(true); only Ok(Ok(true)) confirms.
/// On unconfirmed cleanup: revoke gate AND close the page (bounded).
pub(crate) async fn cleanup(page: &Page, gate: &ProfileGate) -> CleanupResult {
    let call = runtime::CallFunctionOnParams::builder()
        .function_declaration(ABORT_FUNCTION)
        .return_by_value(true)
        .await_promise(true)
        .build();
    let Ok(call) = call else {
        close_page_bounded(page, gate).await;
        return CleanupResult::Unconfirmed;
    };
    let result = timeout(FETCH_TIMEOUT, page.evaluate_function(call)).await;
    match result {
        Ok(Ok(value)) => {
            // Decode {ok: bool, confirmed: bool} via typed extraction.
            let confirmed = value
                .into_value::<serde_json::Value>()
                .ok()
                .and_then(|v| v.get("confirmed").and_then(|c| c.as_bool()))
                .unwrap_or(false);
            if confirmed {
                CleanupResult::Confirmed
            } else {
                close_page_bounded(page, gate).await;
                CleanupResult::Unconfirmed
            }
        }
        Ok(Err(_)) => {
            close_page_bounded(page, gate).await;
            CleanupResult::Unconfirmed
        }
        Err(_) => {
            close_page_bounded(page, gate).await;
            CleanupResult::Unconfirmed
        }
    }
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
