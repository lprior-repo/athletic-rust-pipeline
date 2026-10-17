use super::child::OwnedChild;
use super::error::BrowserStartupError;
use super::BrowserManager;
use crate::clock::Clock;
use crate::drain::DrainReport;
use crate::BrowserSettings;
use chromiumoxide::{Browser, Handler};
use futures::{future::BoxFuture, FutureExt};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

mod drive;

type Ready = Result<Arc<BrowserManager>, BrowserStartupError>;

pub struct BrowserStartup {
    result: oneshot::Receiver<Ready>,
    cancel: CancellationToken,
    armed: bool,
}

impl BrowserStartup {
    pub async fn ready(mut self) -> Ready {
        let result = (&mut self.result)
            .await
            .map_err(|_| BrowserStartupError::ActorStopped)?;
        if result.is_ok() {
            self.armed = false;
        }
        result
    }
}

impl Drop for BrowserStartup {
    fn drop(&mut self) {
        if self.armed {
            self.cancel.cancel();
        }
    }
}

pub(super) enum Prepared {
    Launch(Command),
    Connect(url::Url),
}

enum Phase {
    Prepared(Prepared),
    Discovering,
    Connecting(BoxFuture<'static, Result<(Browser, Handler), BrowserStartupError>>),
    Bootstrapping(BoxFuture<'static, Result<(), BrowserStartupError>>),
    Live,
    Finalized,
}

#[derive(Clone, Debug)]
pub enum BrowserExit {
    Cancelled,
    Failed(BrowserStartupError),
    Panicked,
    Stopped {
        report: DrainReport,
        failure: Option<BrowserStartupError>,
    },
}

#[derive(Clone, Debug)]
pub enum BrowserCleanupAttempt {
    Reaped {
        report: DrainReport,
        failure: Option<BrowserStartupError>,
    },
    StillOwned {
        report: DrainReport,
        failure: BrowserStartupError,
    },
}

pub struct BrowserOwner {
    phase: Phase,
    settings: BrowserSettings,
    clock: Arc<dyn Clock>,
    child: Option<OwnedChild>,
    manager: Option<Arc<BrowserManager>>,
    ready: Option<oneshot::Sender<Ready>>,
    cancel: CancellationToken,
    finalized: Option<BrowserCleanupAttempt>,
    exit: Option<BrowserExit>,
}

impl BrowserOwner {
    pub(super) fn prepare(
        prepared: Prepared,
        settings: BrowserSettings,
        clock: Arc<dyn Clock>,
    ) -> (BrowserStartup, Self) {
        let (ready, result) = oneshot::channel();
        let cancel = CancellationToken::new();
        (
            BrowserStartup {
                result,
                cancel: cancel.clone(),
                armed: true,
            },
            Self {
                phase: Phase::Prepared(prepared),
                settings,
                clock,
                child: None,
                manager: None,
                ready: Some(ready),
                cancel,
                finalized: None,
                exit: None,
            },
        )
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(&mut self, mut stopping: watch::Receiver<bool>) -> BrowserExit {
        if let Some(exit) = &self.exit {
            return exit.clone();
        }
        let cancel = self.cancel.clone();
        let result = {
            let operation = AssertUnwindSafe(self.drive()).catch_unwind();
            tokio::pin!(operation);
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return BrowserExit::Cancelled,
                _ = stopping.wait_for(|stopping| *stopping) => return BrowserExit::Cancelled,
                result = &mut operation => result,
            }
        };
        let exit = match result {
            Ok(Ok(exit)) => exit,
            Ok(Err(error)) => {
                self.notify(Err(error.clone()));
                BrowserExit::Failed(error)
            }
            Err(_) => {
                self.notify(Err(BrowserStartupError::Panicked));
                BrowserExit::Panicked
            }
        };
        self.exit = Some(exit.clone());
        exit
    }

    #[tracing::instrument(skip_all)]
    pub async fn finalize(&mut self) -> BrowserCleanupAttempt {
        if let Some(result) = &self.finalized {
            return result.clone();
        }
        self.cancel.cancel();
        let result = match AssertUnwindSafe(self.cleanup()).catch_unwind().await {
            Ok(result) => result,
            Err(_) => BrowserCleanupAttempt::StillOwned {
                report: DrainReport::default(),
                failure: BrowserStartupError::Panicked,
            },
        };
        if matches!(result, BrowserCleanupAttempt::Reaped { .. }) {
            self.notify(Err(BrowserStartupError::Cancelled));
            self.phase = Phase::Finalized;
            self.finalized = Some(result.clone());
        }
        result
    }

    async fn cleanup(&mut self) -> BrowserCleanupAttempt {
        if let Some(manager) = &self.manager {
            let (report, failure) = manager.shutdown().await;
            let joined = matches!(
                &*manager.join.lock().await,
                super::ActorJoinState::Finished(_)
            );
            return if joined {
                BrowserCleanupAttempt::Reaped { report, failure }
            } else {
                BrowserCleanupAttempt::StillOwned {
                    report,
                    failure: failure
                        .map_or(BrowserStartupError::ActorStopped, core::convert::identity),
                }
            };
        }
        if let Some(child) = self.child.as_mut() {
            if let Err(failure) = child.terminate_and_wait().await {
                return BrowserCleanupAttempt::StillOwned {
                    report: DrainReport::default(),
                    failure,
                };
            }
            drop(self.child.take());
        }
        BrowserCleanupAttempt::Reaped {
            report: DrainReport::default(),
            failure: None,
        }
    }

    fn notify(&mut self, result: Ready) {
        if let Some(ready) = self.ready.take() {
            if ready.send(result).is_err() {
                self.cancel.cancel();
            }
        }
    }
}

#[cfg(test)]
mod tests;
