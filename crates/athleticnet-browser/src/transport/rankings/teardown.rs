use super::rankings_helper::BINDING_NAME;
use crate::{gate::ProfileGate, BrowserError};
use chromiumoxide::cdp::browser_protocol::page::{
    RemoveScriptToEvaluateOnNewDocumentParams, ScriptIdentifier,
};
use chromiumoxide::cdp::js_protocol::runtime::RemoveBindingParams;
use chromiumoxide::Page;
use std::time::Duration;

const TEARDOWN_TIMEOUT: Duration = Duration::from_secs(5);

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
