use std::cell::RefCell;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::time::Duration;

use super::{Capture, GraphAuthorityReferenceV1};
use crate::public_identity_tests::support::TestResult;
use crate::{StoreError, StoreResult};

const TIMEOUT: Duration = Duration::from_secs(30);

pub(in crate::public_identity) struct Gate {
    observed_at: &'static str,
    reached: SyncSender<GraphAuthorityReferenceV1>,
    resume: Receiver<()>,
}

pub(in crate::public_identity) struct Controller {
    reached: Receiver<GraphAuthorityReferenceV1>,
    resume: SyncSender<()>,
}

thread_local! {
    static GATE: RefCell<Option<Gate>> = const { RefCell::new(None) };
}

pub(in crate::public_identity) fn pair(observed_at: &'static str) -> (Gate, Controller) {
    let (reached_tx, reached_rx) = sync_channel(1);
    let (resume_tx, resume_rx) = sync_channel(1);
    (
        Gate {
            observed_at,
            reached: reached_tx,
            resume: resume_rx,
        },
        Controller {
            reached: reached_rx,
            resume: resume_tx,
        },
    )
}

pub(in crate::public_identity) fn with_gate<T>(
    gate: Gate,
    action: impl FnOnce() -> TestResult<T>,
) -> TestResult<T> {
    let previous = GATE.with(|slot| slot.replace(Some(gate)));
    if previous.is_some() {
        drop(GATE.with(|slot| slot.replace(previous)));
        return Err(failure("preparation test gate already installed").into());
    }
    let _reset = Reset;
    action()
}

pub(super) fn observed_at() -> Option<String> {
    GATE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|gate| gate.observed_at.to_owned())
    })
}

pub(in crate::public_identity) fn checkpoint(capture: &Capture) -> StoreResult<()> {
    let Some(gate) = GATE.with(|slot| slot.borrow_mut().take()) else {
        return Ok(());
    };
    if capture.reserved.is_some() || capture.observed_at != gate.observed_at {
        return Err(failure(
            "preparation contender did not retain its distinct unreserved timestamp",
        ));
    }
    gate.reached
        .try_send(capture.graph.clone())
        .map_err(|_| failure("preparation test phase notification"))?;
    gate.resume
        .recv_timeout(TIMEOUT)
        .map_err(|_| failure("preparation test resume timeout or disconnect"))
}

impl Controller {
    pub(in crate::public_identity) fn wait(&self) -> StoreResult<GraphAuthorityReferenceV1> {
        self.reached
            .recv_timeout(TIMEOUT)
            .map_err(|_| failure("preparation test phase not reached"))
    }

    pub(in crate::public_identity) fn resume(self) -> StoreResult<()> {
        self.resume
            .try_send(())
            .map_err(|_| failure("preparation test resume channel"))
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
        drop(GATE.with(|slot| slot.borrow_mut().take()));
    }
}
