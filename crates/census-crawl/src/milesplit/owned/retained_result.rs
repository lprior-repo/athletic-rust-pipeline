use super::{
    parse_owned_meet, read_retained_acquisition, CaptureReadBudget, OwnedAcquisitionRef,
    OwnedPerformance, OwnedRejectionKind, VerifiedOwnedAcquisition, MAX_OWNED_ROWS,
    OWNED_MEET_PHASE,
};
use census_store::{JournalReadBudget, StoreSnapshot};
use chrono::{DateTime, Utc};

mod interpretation;
mod types;

pub use types::{RetainedResultError, VerifiedRetainedOwnedResult};

pub fn read_retained_owned_result(
    view: &StoreSnapshot<'_>,
    reference: &OwnedAcquisitionRef,
    locator: &str,
    budget: CaptureReadBudget,
) -> Result<VerifiedRetainedOwnedResult, RetainedResultError> {
    check_locator(locator)?;
    let acquisition = read_retained_acquisition(view, reference, budget)?;
    let acquired_at = acquisition_time(&acquisition)?;
    let page = parse_owned_meet(acquisition.body(), reference.meet_id())?;
    if page
        .rejected
        .iter()
        .any(|row| row.kind == OwnedRejectionKind::DuplicateResult)
    {
        return Err(RetainedResultError::DuplicateResult);
    }
    let row = page
        .rows
        .into_iter()
        .find(|row| row.locator == locator)
        .ok_or(RetainedResultError::LocatorMissing)?;
    let interpretation_key = interpretation::verify(view, &acquisition, &row)?;
    Ok(VerifiedRetainedOwnedResult::new(
        view.sequence(),
        acquisition,
        acquired_at,
        row,
        interpretation_key,
    ))
}

fn check_locator(locator: &str) -> Result<(), RetainedResultError> {
    let index = locator
        .strip_prefix("data[")
        .and_then(|value| value.strip_suffix(']'))
        .filter(|value| value.len() <= 5)
        .filter(|value| *value == "0" || !value.starts_with('0'))
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|value| value.parse::<usize>().ok());
    match index {
        Some(index) if index < MAX_OWNED_ROWS => Ok(()),
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
