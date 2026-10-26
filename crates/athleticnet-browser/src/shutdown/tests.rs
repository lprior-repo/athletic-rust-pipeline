use crate::actor::{Actor, BrowserOwnership, Command};
use crate::clock::SystemClock;
use crate::gate::ProfileGate;
use crate::request::{RequestAction, RequestSpec};
use crate::{BrowserError, BrowserOutcome, BrowserSettings, BrowserState, BrowserStatus};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "tests/accounting.rs"]
mod accounting;
#[path = "tests/quarantine.rs"]
mod quarantine;

impl Actor {
    pub(crate) fn draining_for_test(
        rx: mpsc::Receiver<Command>,
        handler: tokio::task::JoinHandle<()>,
        reply: oneshot::Sender<crate::drain::DrainReport>,
    ) -> Result<Actor, url::ParseError> {
        let (_handler_tx, handler_rx) = mpsc::channel(1);
        Ok(Actor {
            browser: None,
            child: None,
            ownership: BrowserOwnership::Attached,
            settings: BrowserSettings {
                cdp_endpoint: None,
                executable: PathBuf::from("/no-browser-required"),
                profile_dir: std::env::temp_dir(),
                source_origin: url::Url::parse("https://www.athletic.net/")?,
                follow_same_site_redirects: false,
                tabs: 1,
                request_timeout: Duration::from_secs(1),
                challenge_wait: Duration::from_secs(1),
                headed: true,
            },
            rx,
            handler_rx,
            handler_join: Some(handler),
            handler_stop: CancellationToken::new(),
            jobs: JoinSet::new(),
            pages: Vec::new(),
            pending: VecDeque::new(),
            gate: Arc::new(ProfileGate::new()),
            challenge_latched: false,
            challenge_target: None,
            status: Arc::new(RwLock::new(BrowserStatus {
                state: BrowserState::Ready,
                active_requests: 0,
                tabs: 1,
                cooldown_ms: 0,
            })),
            cooldown_until: Arc::new(Mutex::new(None)),
            shutdown_reply: Some(reply),
            draining: true,
            queue_capacity: 4,
            observers: JoinSet::new(),
            observer_stop: CancellationToken::new(),
            shutdown: CancellationToken::new(),
            recovery_used: false,
            panic_shutdown: false,
            capture_sequence: crate::actor::sequence::CaptureSequence::default(),
            clock: Arc::new(SystemClock),
            region: crate::drain::DrainReport::default(),
            accounting_error: None,
        })
    }
}

#[tokio::test(start_paused = true)]
async fn a_retained_handler_keeps_the_actual_actor_in_quarantine_until_explicit_reap() -> TestResult
{
    let (tx, rx) = mpsc::channel(4);
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(wait_for_handler_release(finished));
    let (reply, first_report) = oneshot::channel();
    let actor = Actor::draining_for_test(rx, handler, reply)?;
    let status = actor.status.clone();
    let join = tokio::spawn(actor.run());
    check!(eq; first_report.await?, crate::drain::DrainReport::default());
    check!(!join.is_finished());
    check!(eq;
        status.read().map_err(|_| "status poisoned")?.state,
        BrowserState::Restarting
    );
    let (reply, answer) = oneshot::channel();
    tx.send(Command::Inspect { reply }).await?;
    check!(eq; answer.await??.state, BrowserState::Restarting);
    let request = RequestSpec {
        url: url::Url::parse("https://www.athletic.net/team/1")?,
        semantic_url: "https://www.athletic.net/team/1".to_owned(),
        action: RequestAction::Fetch {},
        headers: Vec::new(),
    };
    let (reply, answer) = oneshot::channel();
    tx.send(Command::Fetch { request, reply }).await?;
    check!(eq;
        answer.await?,
        BrowserOutcome::failed(BrowserError::Shutdown)
    );
    let (reply, answer) = oneshot::channel();
    tx.send(Command::Restart { reply }).await?;
    check!(matches!(answer.await?, Err(BrowserError::Shutdown)));
    finish
        .send(())
        .map_err(|_| "handler was aborted instead of retained")?;
    let (reply, report) = oneshot::channel();
    tx.send(Command::Shutdown { reply }).await?;
    check!(eq; report.await?, crate::drain::DrainReport::default());
    let exit = join.await?;
    check!(eq; exit.report, crate::drain::DrainReport::default());
    check!(
        matches!(exit.failure, Some(crate::BrowserStartupError::ShutdownFailed(message))
        if message == "browser handler wait timed out")
    );
    check!(eq;
        status.read().map_err(|_| "status poisoned")?.state,
        BrowserState::Restarting
    );
    Ok(())
}

async fn wait_for_handler_release(finished: oneshot::Receiver<()>) {
    assert_eq!(finished.await, Ok(()));
}

fn notify_handler_completed(finished: oneshot::Sender<()>) {
    assert_eq!(finished.send(()), Ok(()));
}
