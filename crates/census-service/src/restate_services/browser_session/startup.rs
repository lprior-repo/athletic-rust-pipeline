use super::BrowserSession;
use crate::outcome::Outcome;
use athleticnet_browser::{
    BrowserCleanupAttempt, BrowserExit, BrowserManager, BrowserOwner, BrowserStartup,
    BrowserStartupError,
};
use futures::FutureExt;
use restate_sdk::prelude::{HandlerError, TerminalError};
use std::panic::AssertUnwindSafe;
use tokio::sync::{oneshot, watch, OwnedMutexGuard};

struct CancelStartup(Option<watch::Sender<bool>>);

impl Drop for CancelStartup {
    fn drop(&mut self) {
        if let Some(cancel) = &self.0 {
            cancel.send_replace(true);
        }
    }
}

impl BrowserSession {
    pub(super) async fn start_owned(&self) -> Result<(), HandlerError> {
        let admission = self.launch_admission().await;
        let (startup, owner) = self.prepare_startup()?;
        let reservation = self
            .region
            .reserve_browser()
            .map_err(|error| TerminalError::new(format!("browser {error}")))?;
        let (published, received) = oneshot::channel();
        let (cancel, cancelled) = watch::channel(false);
        let mut cancellation = CancelStartup(Some(cancel.clone()));
        let session = self.clone();
        let stopping = self.region.stopping();
        let work = self.region.dedicated(reservation, move || {
            let _cancellation = cancel;
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("browser owner runtime: {error}"))?;
            runtime.block_on(
                session.run_owned(startup, owner, admission, published, cancelled, stopping),
            )
        });
        tokio::pin!(work);
        tokio::select! {
            biased;
            result = received => {
                result.map_err(|_| TerminalError::new("browser owner ended before publication"))?
                    .map_err(TerminalError::new)?;
                cancellation.0 = None;
                Ok(())
            }
            result = &mut work => Err(TerminalError::new(match result {
                Ok(Outcome::Err(error)) => error,
                Ok(Outcome::Cancelled) => "browser owner registration cancelled".to_owned(),
                Ok(Outcome::Panicked) => "browser owner thread panicked".to_owned(),
                Ok(Outcome::Ok(())) => "browser owner exited before readiness".to_owned(),
                Err(error) => format!("browser owner registration failed: {error}"),
            }).into()),
        }
    }

    fn prepare_startup(&self) -> Result<(BrowserStartup, BrowserOwner), HandlerError> {
        match self.settings.cdp_endpoint.clone() {
            Some(endpoint) => BrowserManager::prepare_connect(
                endpoint,
                (*self.settings).clone(),
                self.clock.clone(),
            ),
            None => BrowserManager::prepare_launch((*self.settings).clone(), self.clock.clone()),
        }
        .map_err(|error| TerminalError::new(error.to_string()).into())
    }

    async fn run_owned(
        &self,
        startup: BrowserStartup,
        mut owner: BrowserOwner,
        admission: OwnedMutexGuard<()>,
        published: oneshot::Sender<Result<(), String>>,
        cancel: watch::Receiver<bool>,
        stopping: watch::Receiver<bool>,
    ) -> Result<(), String> {
        let epoch = self
            .begin_launch()
            .await
            .map_err(|error| format!("{error:?}"))?;
        let mut published = Some(published);
        let exit = match AssertUnwindSafe(self.operate(
            &mut owner,
            startup,
            epoch,
            &mut published,
            cancel,
            stopping,
        ))
        .catch_unwind()
        .await
        {
            Ok(exit) => exit,
            Err(_) => BrowserExit::Panicked,
        };
        let failure = self.finish_owned(&mut owner, epoch, &exit).await;
        if let Some(published) = published.take() {
            let error = match &failure {
                Some(error) => error.clone(),
                None => format!("browser startup ended before publication: {exit:?}"),
            };
            if published.send(Err(error)).is_err() {
                tracing::debug!("browser startup error receiver dropped after physical cleanup");
            }
        }
        drop(admission);
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    async fn operate(
        &self,
        owner: &mut BrowserOwner,
        startup: BrowserStartup,
        epoch: u64,
        published: &mut Option<oneshot::Sender<Result<(), String>>>,
        mut cancel: watch::Receiver<bool>,
        stopping: watch::Receiver<bool>,
    ) -> BrowserExit {
        let running = owner.run(stopping);
        tokio::pin!(running);
        let publication = tokio::select! {
            biased;
            _ = cancel.wait_for(|cancelled| *cancelled) => return BrowserExit::Cancelled,
            exit = &mut running => return exit,
            result = self.publish(startup, epoch, published) => result,
        };
        if let Err(error) = publication {
            return BrowserExit::Failed(error);
        }
        tokio::select! {
            biased;
            _ = cancel.wait_for(|cancelled| *cancelled) => BrowserExit::Cancelled,
            exit = &mut running => exit,
        }
    }

    async fn publish(
        &self,
        startup: BrowserStartup,
        epoch: u64,
        published: &mut Option<oneshot::Sender<Result<(), String>>>,
    ) -> Result<(), BrowserStartupError> {
        let manager = startup.ready().await?;
        let mut lane = self.lane.lock().await;
        if lane.epoch != epoch || !lane.starting || lane.manager.is_some() {
            return Err(BrowserStartupError::ShutdownFailed(
                "browser publication epoch changed".to_owned(),
            ));
        }
        lane.manager = Some(manager);
        lane.starting = false;
        drop(lane);
        if let Some(published) = published.take() {
            if published.send(Ok(())).is_err() {
                return Err(BrowserStartupError::Cancelled);
            }
        }
        Ok(())
    }

    async fn finish_owned(
        &self,
        owner: &mut BrowserOwner,
        epoch: u64,
        exit: &BrowserExit,
    ) -> Option<String> {
        loop {
            match owner.finalize().await {
                BrowserCleanupAttempt::Reaped { report, failure } => {
                    tracing::info!(?report, ?failure, ?exit, "browser lifecycle region joined");
                    let error = failure
                        .map(|error| error.to_string())
                        .or_else(|| match exit {
                            BrowserExit::Failed(error) => Some(error.to_string()),
                            BrowserExit::Panicked => Some("browser lifecycle panicked".to_owned()),
                            BrowserExit::Stopped { failure, .. } => {
                                failure.as_ref().map(ToString::to_string)
                            }
                            BrowserExit::Cancelled => None,
                        });
                    let mut lane = self.lane.lock().await;
                    if lane.epoch == epoch {
                        lane.manager = None;
                        lane.starting = false;
                        lane.failure = error.clone();
                    }
                    return error;
                }
                BrowserCleanupAttempt::StillOwned { report, failure } => {
                    tracing::error!(?report, %failure, "browser worker retains physical cleanup");
                    let mut lane = self.lane.lock().await;
                    if lane.epoch == epoch {
                        lane.failure = Some(format!("StillOwned: {failure}"));
                    }
                    drop(lane);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}
