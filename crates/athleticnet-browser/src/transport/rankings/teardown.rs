//! Undoing what one rankings attempt installed on its page.
//!
//! A capture attempt leaves three things behind — a patched `window.fetch`, the interceptor
//! script, and the binding that delivers the payload — and every exit path has to remove them
//! before the page is reused. Each removal here is bounded and reports its failure upward instead
//! of leaving a half-cleaned page behind: the caller's deadline cancels the whole attempt, so the
//! teardown may be the only code that still runs for it.
//!
//! `shutdown_capture` takes the pieces rather than the attempt struct, so this module never has to
//! see `CaptureRun`'s other fields.

use super::rankings_helper::BINDING_NAME;
use crate::{gate::ProfileGate, BrowserError};
use chromiumoxide::cdp::browser_protocol::page::{
    RemoveScriptToEvaluateOnNewDocumentParams, ScriptIdentifier,
};
use chromiumoxide::cdp::js_protocol::runtime::RemoveBindingParams;
use chromiumoxide::Page;
use std::time::Duration;

/// Bound for one teardown step.
///
/// Cleanup runs on a page whose deadline has already passed, so it may not wait indefinitely: a
/// wedged renderer would otherwise turn a failed attempt into a hung drain.
const TEARDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Run capture cleanup, revoking admission and closing the page when it fails.
///
/// A page whose cleanup failed may still be fetching under the injected script, so it is removed
/// from service — admission revoked, page closed — and the original cleanup error is returned.
pub(super) async fn shutdown_capture(
    page: &Page,
    gate: &ProfileGate,
    script_id: Option<&ScriptIdentifier>,
    binding_attempted: bool,
) -> Result<(), BrowserError> {
    let cleanup = tokio::time::timeout(
        TEARDOWN_TIMEOUT,
        cleanup_capture(page, script_id, binding_attempted),
    )
    .await;
    match cleanup {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            gate.revoke();
            close_failed_page(page).await?;
            Err(error)
        }
        Err(_) => {
            gate.revoke();
            close_failed_page(page).await?;
            Err(BrowserError::Timeout)
        }
    }
}

/// Close a page whose capture cleanup failed, bounded like the cleanup itself.
async fn close_failed_page(page: &Page) -> Result<(), BrowserError> {
    tokio::time::timeout(
        TEARDOWN_TIMEOUT,
        page.execute(chromiumoxide::cdp::browser_protocol::page::CloseParams::default()),
    )
    .await
    .map_err(|_| BrowserError::Timeout)?
    .map_err(|_| BrowserError::Transport)?;
    Ok(())
}

/// Remove the capture hooks, keeping the first failure and attempting every removal.
///
/// Each step is tried even after an earlier one failed: a page that keeps a stale interceptor but
/// loses its binding fails differently from one that keeps both, and the caller's decision does
/// not depend on which step broke.
async fn cleanup_capture(
    page: &Page,
    script_id: Option<&ScriptIdentifier>,
    binding_attempted: bool,
) -> Result<(), BrowserError> {
    let mut cleanup_error = None;
    if page
        .evaluate(
            "if (typeof window.__RANKINGS_ORIGINAL_FETCH === 'function') { window.fetch = window.__RANKINGS_ORIGINAL_FETCH; } delete window.__RANKINGS_ORIGINAL_FETCH; delete window.retainRankingResponse;",
        )
        .await
        .is_err()
    {
        tracing::error!("rankings cleanup failed while restoring fetch");
        cleanup_error = Some(BrowserError::Transport);
    }
    if let Some(identifier) = script_id {
        let params = RemoveScriptToEvaluateOnNewDocumentParams::builder()
            .identifier(identifier.clone())
            .build()
            .map_err(|_| BrowserError::Protocol);
        match params {
            Ok(params) => {
                if page.execute(params).await.is_err() {
                    tracing::error!("rankings cleanup failed while removing script");
                    if cleanup_error.is_none() {
                        cleanup_error = Some(BrowserError::Transport);
                    }
                }
            }
            Err(error) => {
                tracing::error!("rankings cleanup failed while building script removal");
                if cleanup_error.is_none() {
                    cleanup_error = Some(error);
                }
            }
        }
    }
    if binding_attempted
        && page
            .execute(RemoveBindingParams::new(BINDING_NAME))
            .await
            .is_err()
    {
        tracing::error!("rankings cleanup failed while removing binding");
        if cleanup_error.is_none() {
            cleanup_error = Some(BrowserError::Transport);
        }
    }
    cleanup_error.map_or(Ok(()), Err)
}
