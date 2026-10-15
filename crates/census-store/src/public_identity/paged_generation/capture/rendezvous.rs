use std::cell::Cell;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::time::Duration;

use crate::{StoreError, StoreResult};

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    AfterReservation,
    AfterSeal,
}

pub(super) struct Gate {
    phase: Phase,
    reached: SyncSender<()>,
    resume: Receiver<Directive>,
}

pub(super) struct Controller {
    reached: Receiver<()>,
    resume: SyncSender<Directive>,
}

enum Directive {
    Continue,
    #[cfg(unix)]
    Abort,
}

thread_local! {
    static GATE: Cell<Option<Gate>> = const { Cell::new(None) };
}

pub(super) fn pair(phase: Phase) -> (Gate, Controller) {
    let (reached_tx, reached_rx) = sync_channel(1);
    let (resume_tx, resume_rx) = sync_channel(1);
    (
        Gate {
            phase,
            reached: reached_tx,
            resume: resume_rx,
        },
        Controller {
            reached: reached_rx,
            resume: resume_tx,
        },
    )
}

pub(super) fn with_gate<T>(gate: Gate, action: impl FnOnce() -> StoreResult<T>) -> StoreResult<T> {
    let previous = GATE.with(|slot| slot.replace(Some(gate)));
    if previous.is_some() {
        GATE.with(|slot| slot.set(previous));
        return Err(failure("capture test gate already installed"));
    }
    let _reset = Reset;
    action()
}

pub(super) fn checkpoint(phase: Phase) -> StoreResult<()> {
    match GATE.with(Cell::take) {
        Some(gate) if gate.phase == phase => {
            gate.reached
                .try_send(())
                .map_err(|_| failure("capture test phase notification"))?;
            match gate
                .resume
                .recv_timeout(TIMEOUT)
                .map_err(|_| failure("capture test phase resume timeout or disconnect"))?
            {
                Directive::Continue => Ok(()),
                #[cfg(unix)]
                Directive::Abort => std::process::abort(),
            }
        }
        other => {
            GATE.with(|slot| slot.set(other));
            Ok(())
        }
    }
}

impl Controller {
    pub(super) fn wait(&self) -> StoreResult<()> {
        self.reached
            .recv_timeout(TIMEOUT)
            .map_err(|_| failure("capture test phase not reached"))
    }

    #[cfg(unix)]
    pub(super) fn abort(self) -> StoreResult<()> {
        self.resume
            .try_send(Directive::Abort)
            .map_err(|_| failure("capture test abort channel"))
    }

    pub(super) fn resume(self) -> StoreResult<()> {
        self.resume
            .try_send(Directive::Continue)
            .map_err(|_| failure("capture test resume channel"))
    }
}

fn failure(detail: &'static str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        drop(GATE.with(Cell::take));
    }
}
