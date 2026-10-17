use super::{
    gate::ProfileGate,
    lifecycle::ActorExit,
    navigation::NavigationOutcome,
    pool::{PageSlot, Pending},
    BrowserError, BrowserOutcome, BrowserResponse, BrowserSettings, BrowserState, BrowserStatus,
};
use crate::lifecycle::error::BrowserStartupError;
use crate::{
    clock::Clock,
    drain::{DrainAccountingError, DrainReport},
};
use chromiumoxide::Browser;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock},
};
use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinHandle, JoinSet},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
mod accounting;
#[path = "lifecycle_ops.rs"]
mod lifecycle_ops;
mod sequence;
#[path = "shutdown.rs"]
mod shutdown;
#[path = "state.rs"]
mod state;
use sequence::CaptureSequence;

const QUEUE_MULTIPLIER: usize = 4;

pub(super) struct Actor {
    pub(super) browser: Option<Browser>,
    pub(super) child: Option<crate::lifecycle::child::OwnedChild>,
    pub(super) ownership: BrowserOwnership,
    pub(super) settings: BrowserSettings,
    pub(super) rx: mpsc::Receiver<Command>,
    pub(super) handler_rx: mpsc::Receiver<HandlerEvent>,
    pub(super) handler_join: Option<JoinHandle<()>>,
    pub(super) handler_stop: CancellationToken,
    pub(super) jobs: JoinSet<JobResult>,
    pub(super) pages: Vec<PageSlot>,
    pub(super) pending: VecDeque<Pending>,
    pub(super) gate: Arc<ProfileGate>,
    pub(super) challenge_latched: bool,
    pub(super) challenge_target: Option<ChallengeTarget>,
    pub(super) status: Arc<RwLock<BrowserStatus>>,
    pub(super) cooldown_until: Arc<Mutex<Option<Instant>>>,
    pub(super) shutdown_reply: Option<oneshot::Sender<DrainReport>>,
    pub(super) draining: bool,
    pub(super) queue_capacity: usize,
    pub(super) observers: JoinSet<Result<(), BrowserError>>,
    pub(super) observer_stop: CancellationToken,
    pub(super) shutdown: CancellationToken,
    pub(super) recovery_used: bool,
    pub(super) panic_shutdown: bool,
    capture_sequence: CaptureSequence,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) region: DrainReport,
    pub(super) accounting_error: Option<DrainAccountingError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BrowserOwnership {
    Launched,
    Attached,
}
pub(super) struct ChallengeTarget {
    pub(super) url: url::Url,
    pub(super) post: bool,
}

pub(super) enum Command {
    Bootstrap {
        reply: oneshot::Sender<Result<(), BrowserStartupError>>,
    },
    Fetch {
        request: crate::request::RequestSpec,
        reply: oneshot::Sender<BrowserOutcome>,
    },
    Inspect {
        reply: oneshot::Sender<Result<BrowserStatus, BrowserError>>,
    },
    Recover {
        reply: oneshot::Sender<Result<BrowserStatus, BrowserError>>,
    },
    Restart {
        reply: oneshot::Sender<Result<BrowserStatus, BrowserError>>,
    },
    Shutdown {
        reply: oneshot::Sender<DrainReport>,
    },
}

pub(super) enum HandlerEvent {
    Failed,
}

pub(super) struct JobResult {
    pub(super) slot: usize,
    pub(super) request: crate::request::RequestSpec,
    pub(super) reply: oneshot::Sender<BrowserOutcome>,
    pub(super) result: Result<BrowserResponse, BrowserError>,
}

pub(super) struct ActorHandles {
    pub(super) status: Arc<RwLock<BrowserStatus>>,
    pub(super) cooldown_until: Arc<Mutex<Option<Instant>>>,
    pub(super) gate: Arc<ProfileGate>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) handler_stop: CancellationToken,
}

impl Actor {
    pub(super) fn new(
        browser: (Browser, BrowserOwnership),
        settings: BrowserSettings,
        rx: mpsc::Receiver<Command>,
        handler_pair: (mpsc::Receiver<HandlerEvent>, JoinHandle<()>),
        handles: ActorHandles,
    ) -> Self {
        let ActorHandles {
            status,
            cooldown_until,
            gate,
            clock,
            handler_stop,
        } = handles;
        let queue_capacity = settings.tabs.saturating_mul(QUEUE_MULTIPLIER).max(1);
        Self {
            browser: Some(browser.0),
            child: None,
            ownership: browser.1,
            settings,
            rx,
            handler_rx: handler_pair.0,
            handler_join: Some(handler_pair.1),
            handler_stop,
            jobs: JoinSet::new(),
            pages: Vec::new(),
            pending: VecDeque::new(),
            gate,
            challenge_latched: false,
            challenge_target: None,
            recovery_used: false,
            status,
            cooldown_until,
            shutdown_reply: None,
            draining: false,
            queue_capacity,
            observers: JoinSet::new(),
            observer_stop: CancellationToken::new(),
            shutdown: CancellationToken::new(),
            panic_shutdown: false,
            capture_sequence: CaptureSequence::default(),
            clock,
            region: DrainReport::default(),
            accounting_error: None,
        }
    }

    pub(super) fn next_capture_nonce(&mut self) -> Result<u64, BrowserError> {
        self.capture_sequence.issue()
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn run(mut self) -> ActorExit {
        use futures::FutureExt;
        use std::panic::AssertUnwindSafe;
        match AssertUnwindSafe(self.run_loop()).catch_unwind().await {
            Ok(exit) => exit,
            Err(_) => self.finalize_panicked().await,
        }
    }

    async fn finalize_panicked(&mut self) -> ActorExit {
        use futures::FutureExt;
        use std::panic::AssertUnwindSafe;
        self.draining = true;
        self.shutdown.cancel();
        self.observer_stop.cancel();
        self.gate.revoke();
        self.reject_pending(BrowserError::Shutdown);
        loop {
            match AssertUnwindSafe(self.close_browser()).catch_unwind().await {
                Ok((report, cleanup)) => {
                    let failure = Some(match cleanup {
                        Some(error) => {
                            BrowserStartupError::ShutdownFailed(format!("actor panicked; {error}"))
                        }
                        None => BrowserStartupError::Panicked,
                    });
                    let exit = ActorExit { report, failure };
                    self.finish_shutdown(&exit);
                    return exit;
                }
                Err(_) => {
                    tracing::error!("browser panic cleanup still owns physical resources");
                    tokio::time::sleep(crate::SHUTDOWN_TIMEOUT).await;
                }
            }
        }
    }

    async fn run_loop(&mut self) -> ActorExit {
        loop {
            if self.panic_shutdown || (self.draining && self.jobs.is_empty()) {
                self.panic_shutdown = false;
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.gate.revoke();
                self.reject_pending(BrowserError::Shutdown);
                let (report, failure) = self.close_browser().await;
                let exit = ActorExit { report, failure };
                self.finish_shutdown(&exit);
                return exit;
            }
            tokio::select! {
                command = self.rx.recv() => self.command(command).await,
                event = self.handler_rx.recv() => {
                    if event.is_none() || matches!(event, Some(HandlerEvent::Failed)) {
                        return self.handler_stopped().await;
                    }
                }
                job = self.jobs.join_next(), if !self.jobs.is_empty() => self.complete_job(job),
                observer = self.observers.join_next(), if !self.observers.is_empty() => self.complete_observer(observer),
                _ = self.gate.closed(), if {
                    !self.challenge_latched
                        && !self.shutdown.is_cancelled()
                        && !self.pages.is_empty()
                        && self.status.read().ok().is_none_or(|status| status.state != BrowserState::CoolingDown)
                } => {
                    self.latch_challenge(None);
                }
            }
            self.schedule_pending();
        }
    }
}
