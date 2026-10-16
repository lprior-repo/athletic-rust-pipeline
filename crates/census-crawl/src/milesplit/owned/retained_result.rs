use super::{
    CaptureReadBudget, OwnedAcquisitionRef, OwnedPerformance, VerifiedOwnedAcquisition,
    MAX_OWNED_ROWS, OWNED_MEET_PHASE,
};
use crate::milesplit::RetainedJournalView;
use census_store::{JournalReadBudget, StoreSnapshot};
use chrono::{DateTime, Utc};

mod interpretation;
mod session;
mod types;
pub(in crate::milesplit) use session::{OwnedResultSession, MAX_SESSION_CAPTURES};

pub use types::{RetainedResultError, VerifiedRetainedOwnedResult};

pub fn read_retained_owned_result(
    view: &StoreSnapshot<'_>,
    reference: &OwnedAcquisitionRef,
    locator: &str,
    budget: CaptureReadBudget,
) -> Result<VerifiedRetainedOwnedResult, RetainedResultError> {
    read_retained_owned_result_from(view, reference, locator, budget)
}

pub fn read_retained_owned_result_from(
    view: &impl RetainedJournalView,
    reference: &OwnedAcquisitionRef,
    locator: &str,
    budget: CaptureReadBudget,
) -> Result<VerifiedRetainedOwnedResult, RetainedResultError> {
    OwnedResultSession::new(view, budget).read(reference, locator)
}

fn check_locator(locator: &str) -> Result<usize, RetainedResultError> {
    let index = locator
        .strip_prefix("data[")
        .and_then(|value| value.strip_suffix(']'))
        .filter(|value| value.len() <= 5)
        .filter(|value| *value == "0" || !value.starts_with('0'))
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|value| value.parse::<usize>().ok());
    match index {
        Some(index) if index < MAX_OWNED_ROWS => Ok(index),
        _ => Err(RetainedResultError::InvalidLocator),
    }
}

fn acquisition_time(
    acquisition: &VerifiedOwnedAcquisition,
) -> Result<DateTime<Utc>, RetainedResultError> {
    let capture = acquisition.capture();
    if capture.status != 200 {
        return Err(RetainedResultError::HttpRefused {
            status: capture.status,
        });
    }
    DateTime::parse_from_rfc3339(&capture.fetched_at)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| RetainedResultError::InvalidAcquisitionTime)
}
