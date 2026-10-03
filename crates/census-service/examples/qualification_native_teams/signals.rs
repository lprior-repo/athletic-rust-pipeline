use anyhow::{Context, Result};
use tokio::signal::unix::{signal, Signal, SignalKind};

#[cfg(test)]
mod tests;

pub(super) struct ShutdownSignals {
    term: Signal,
    interrupt: Signal,
}

impl ShutdownSignals {
    pub(super) fn install() -> Result<Self> {
        Ok(Self {
            term: signal(SignalKind::terminate()).context("installing driver TERM ownership")?,
            interrupt: signal(SignalKind::interrupt())
                .context("installing driver INT ownership")?,
        })
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn requested(&mut self) -> Result<()> {
        tokio::select! {
            signal = self.term.recv() => signal.context("driver TERM listener closed"),
            signal = self.interrupt.recv() => signal.context("driver INT listener closed"),
        }
    }
}
