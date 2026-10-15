use super::Interruption;
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use tokio::signal::unix::{signal, Signal, SignalKind};

pub struct Interrupts {
    runtime: Runtime,
    interrupt: Signal,
    terminate: Signal,
}

impl Interrupts {
    pub fn register() -> Result<Self> {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()?;
        let context = runtime.enter();
        let interrupt = signal(SignalKind::interrupt())?;
        let terminate = signal(SignalKind::terminate())?;
        drop(context);
        Ok(Self {
            runtime,
            interrupt,
            terminate,
        })
    }

    pub fn wait(&mut self, pause: Duration) -> Result<Option<Interruption>> {
        let Self {
            runtime,
            interrupt,
            terminate,
        } = self;
        runtime.block_on(async {
            tokio::select! {
                biased;
                value = terminate.recv() => value
                    .context("termination signal stream closed")
                    .map(|()| Some(Interruption::Terminate)),
                value = interrupt.recv() => value
                    .context("interrupt signal stream closed")
                    .map(|()| Some(Interruption::Interrupt)),
                () = tokio::time::sleep(pause) => Ok(None),
            }
        })
    }
}
