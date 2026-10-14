use super::fixtures::TestResult;
use census_store::{InputRecord, InputState, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetainedState {
    Missing,
    Requested,
    Freezing,
    Frozen,
    Abandoned,
}

impl RetainedState {
    fn record_state(self) -> Option<InputState> {
        match self {
            Self::Missing => None,
            Self::Requested => Some(InputState::Requested),
            Self::Freezing => Some(InputState::Freezing),
            Self::Frozen => Some(InputState::Frozen),
            Self::Abandoned => Some(InputState::Abandoned),
        }
    }
}

pub(super) fn install_record(
    store: &Store,
    attempt: &str,
    bytes: u64,
    inventory: &[String],
    state: RetainedState,
) -> TestResult<Option<InputRecord>> {
    if state != RetainedState::Missing {
        transition_record(store, attempt, bytes, inventory, state)?;
    }
    let record = store.get_input_record(attempt)?;
    check!(eq; record.as_ref().map(InputRecord::state), state.record_state());
    check!(eq; store.get_lease_record(attempt)?.is_some(), state != RetainedState::Missing);
    Ok(record)
}

fn transition_record(
    store: &Store,
    attempt: &str,
    bytes: u64,
    inventory: &[String],
    state: RetainedState,
) -> TestResult {
    let record = InputRecord::new(attempt, bytes, inventory)?;
    store.put_input_record(&record)?;
    let (epoch, claimed) = store.input_lease_acquire(
        attempt,
        record.revision(),
        "version-preservation-fixture",
        "2999-12-31",
    )?;
    match state {
        RetainedState::Requested => {}
        RetainedState::Freezing | RetainedState::Frozen => {
            let freezing = store.input_freeze(attempt, claimed.revision, epoch, inventory)?;
            if state == RetainedState::Frozen {
                store.input_frozen(attempt, freezing.revision, epoch, bytes, inventory)?;
            }
        }
        RetainedState::Abandoned => {
            store.input_abandon(attempt, claimed.revision, epoch)?;
        }
        RetainedState::Missing => return Err("missing fixtures must not create an input".into()),
    }
    Ok(())
}
