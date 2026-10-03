use anyhow::{ensure, Context, Result};
use std::cell::RefCell;
use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

thread_local! {
    static CURRENT: RefCell<Option<Cancellation>> = const { RefCell::new(None) };
    static CLEANUP: RefCell<bool> = const { RefCell::new(false) };
}

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicI32>);

impl Cancellation {
    pub fn check(&self) -> Result<()> {
        let reason = self.0.load(Ordering::Acquire);
        ensure!(
            reason == 0,
            "qualification cancelled: signal/resource/deadline reason={reason}"
        );
        Ok(())
    }

    pub fn fail_resource(&self) {
        match self
            .0
            .compare_exchange(0, -1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) | Err(_) => {}
        }
    }

    pub fn reason(&self) -> i32 {
        self.0.load(Ordering::Acquire)
    }
}

pub fn current() -> Cancellation {
    CURRENT.with(|value| match value.borrow().as_ref() {
        Some(token) => token.clone(),
        None => Cancellation::default(),
    })
}

pub fn checkpoint() -> Result<()> {
    if CLEANUP.with(|value| *value.borrow()) {
        return Ok(());
    }
    current().check()
}

pub fn pause(duration: Duration) -> Result<()> {
    ensure!(
        duration <= Duration::from_secs(10),
        "pause exceeds cancellation budget"
    );
    let ticks = duration.as_millis().div_ceil(100);
    (0..ticks).try_for_each(|_| {
        checkpoint()?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    })
}

pub fn cleanup<T>(action: impl FnOnce() -> T) -> T {
    let previous = CLEANUP.with(|value| value.replace(true));
    let result = action();
    CLEANUP.with(|value| value.replace(previous));
    result
}

pub struct Signals {
    token: Cancellation,
    previous: Option<Cancellation>,
    finish: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Signals {
    pub fn install() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (terminate, interrupt) = {
            let _entered = runtime.enter();
            (
                signal(SignalKind::terminate())?,
                signal(SignalKind::interrupt())?,
            )
        };
        let token = Cancellation::default();
        let worker_token = token.clone();
        let (finish, finished) = oneshot::channel();
        let worker = std::thread::Builder::new()
            .name("qualification-signals".into())
            .spawn(move || {
                runtime.block_on(listen(terminate, interrupt, worker_token, finished))
            })?;
        let previous = CURRENT.with(|value| value.replace(Some(token.clone())));
        Ok(Self {
            token,
            previous,
            finish: Some(finish),
            worker: Some(worker),
        })
    }

    pub fn reason(&self) -> i32 {
        self.token.reason()
    }

    pub fn finish(&mut self) -> Result<()> {
        drop(CURRENT.with(|value| value.replace(self.previous.take())));
        let delivered = self.finish.take().map_or(Ok(()), |finish| {
            finish
                .send(())
                .map_err(|()| anyhow::anyhow!("signal worker stopped before owned finalization"))
        });
        let joined = match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|_| anyhow::anyhow!("signal worker panicked"))?,
            None => Ok(()),
        };
        joined?;
        delivered?;
        Ok(())
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        if self.worker.is_some() {
            if let Err(error) = self.finish() {
                eprintln!("signal region cleanup failed: {error:#}");
            }
        }
    }
}

async fn listen(
    mut terminate: tokio::signal::unix::Signal,
    mut interrupt: tokio::signal::unix::Signal,
    token: Cancellation,
    mut finish: oneshot::Receiver<()>,
) -> Result<()> {
    tokio::select! {
        result = &mut finish => return result.context("signal region owner disconnected"),
        value = terminate.recv() => {
            ensure!(value.is_some(), "TERM signal stream closed");
            token.0.store(15, Ordering::Release);
        },
        value = interrupt.recv() => {
            ensure!(value.is_some(), "INT signal stream closed");
            token.0.store(2, Ordering::Release);
        },
        _ = tokio::time::sleep(Duration::from_secs(3600)) => token.0.store(-2, Ordering::Release),
    }
    tokio::time::timeout(Duration::from_secs(400), finish)
        .await
        .context("signal owner cleanup exceeded 400 seconds")?
        .context("signal region owner disconnected")
}
