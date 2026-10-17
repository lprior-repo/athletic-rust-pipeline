use crate::lifecycle::child::OwnedChild;
use crate::lifecycle::error::BrowserStartupError;
use crate::SHUTDOWN_TIMEOUT;
use chromiumoxide::Browser;

pub(super) enum ClosePhase {
    Graceful,
    ReapOnly,
}

pub(super) enum CloseAttempt {
    Reaped,
    StillOwned(BrowserStartupError),
}

pub(super) async fn close(
    browser: Option<&mut Browser>,
    child: &mut OwnedChild,
    phase: ClosePhase,
) -> CloseAttempt {
    let close_error = match phase {
        ClosePhase::Graceful => match browser {
            Some(browser) => close_request(browser).await.err(),
            None => Some(BrowserStartupError::ShutdownFailed(
                "owned browser has no CDP connection".to_owned(),
            )),
        },
        ClosePhase::ReapOnly => None,
    };
    let result = if close_error.is_some() {
        child.terminate_and_wait().await
    } else {
        child.wait().await
    };
    match (result, close_error) {
        (Ok(()), None) => CloseAttempt::Reaped,
        (Ok(()), Some(error)) => CloseAttempt::StillOwned(error),
        (Err(error), None) => CloseAttempt::StillOwned(error),
        (Err(error), Some(close)) => CloseAttempt::StillOwned(BrowserStartupError::ShutdownFailed(
            format!("{close}; {error}"),
        )),
    }
}

async fn close_request(browser: &mut Browser) -> Result<(), BrowserStartupError> {
    tokio::time::timeout(SHUTDOWN_TIMEOUT, browser.close())
        .await
        .map_err(|_| BrowserStartupError::ShutdownFailed("browser close timed out".to_string()))?
        .map_err(|error| {
            BrowserStartupError::ShutdownFailed(format!("browser close failed: {error}"))
        })?;
    Ok(())
}

pub(super) enum HandlerAttempt {
    Finished(Result<(), BrowserStartupError>),
    StillOwned(BrowserStartupError),
}

pub(super) async fn join_handler(handle: &mut tokio::task::JoinHandle<()>) -> HandlerAttempt {
    match tokio::time::timeout(SHUTDOWN_TIMEOUT, handle).await {
        Ok(Ok(())) => HandlerAttempt::Finished(Ok(())),
        Ok(Err(error)) => HandlerAttempt::Finished(Err(BrowserStartupError::ShutdownFailed(
            format!("browser handler join failed: {error}"),
        ))),
        Err(_) => HandlerAttempt::StillOwned(BrowserStartupError::ShutdownFailed(
            "browser handler wait timed out".to_string(),
        )),
    }
}
