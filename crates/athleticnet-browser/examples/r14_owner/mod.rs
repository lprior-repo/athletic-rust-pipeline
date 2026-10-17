use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::{BrowserCleanupAttempt, BrowserExit, BrowserManager, BrowserSettings};
use std::error::Error;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::watch;

type ProbeResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub(crate) struct ProbeSupervisor {
    stopping: watch::Sender<bool>,
    thread: Option<JoinHandle<Result<(), String>>>,
}

pub(crate) async fn launch(
    settings: BrowserSettings,
) -> ProbeResult<(Arc<BrowserManager>, ProbeSupervisor)> {
    let (startup, mut owner) = BrowserManager::prepare_launch(settings, Arc::new(SystemClock))?;
    let (stopping, stopped) = watch::channel(false);
    let thread = std::thread::Builder::new()
        .name("r14-browser-owner".to_owned())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime.block_on(async {
                let exit = owner.run(stopped).await;
                loop {
                    match owner.finalize().await {
                        BrowserCleanupAttempt::Reaped { report, failure } => {
                            println!(
                                "OWNER_JOINED exit={exit:?} report={report:?} failure={failure:?}"
                            );
                            return match failure {
                                Some(error) => Err(error.to_string()),
                                None => match exit {
                                    BrowserExit::Failed(error) => Err(error.to_string()),
                                    BrowserExit::Panicked => {
                                        Err("browser owner panicked".to_owned())
                                    }
                                    BrowserExit::Stopped {
                                        failure: Some(error),
                                        ..
                                    } => Err(error.to_string()),
                                    BrowserExit::Cancelled
                                    | BrowserExit::Stopped { failure: None, .. } => Ok(()),
                                },
                            };
                        }
                        BrowserCleanupAttempt::StillOwned { report, failure } => {
                            eprintln!("OWNER_STILL_OWNED report={report:?} failure={failure}");
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            })
        })?;
    let mut supervisor = ProbeSupervisor {
        stopping,
        thread: Some(thread),
    };
    match startup.ready().await {
        Ok(manager) => Ok((manager, supervisor)),
        Err(error) => {
            let cleanup = supervisor.stop_and_join();
            Err(match cleanup {
                Ok(()) => error.to_string(),
                Err(cleanup) => format!("{error}; {cleanup}"),
            }
            .into())
        }
    }
}

impl ProbeSupervisor {
    pub(crate) fn stop_and_join(&mut self) -> Result<(), String> {
        self.stopping.send_replace(true);
        match self.thread.take() {
            Some(thread) => thread
                .join()
                .map_err(|_| "browser owner thread panicked".to_owned())?,
            None => Ok(()),
        }
    }
}

impl Drop for ProbeSupervisor {
    fn drop(&mut self) {
        if let Err(error) = self.stop_and_join() {
            eprintln!("OWNER_JOIN_FAILED {error}");
        }
    }
}
