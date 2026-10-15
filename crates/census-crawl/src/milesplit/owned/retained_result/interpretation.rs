use super::{
    JournalReadBudget, OwnedPerformance, RetainedResultError, StoreSnapshot,
    VerifiedOwnedAcquisition, OWNED_MEET_PHASE,
};
use census_domain::model::serialized_digest;
use serde_json::Value;

const MAX_INTERPRETATION_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn verify(
    view: &StoreSnapshot<'_>,
    acquisition: &VerifiedOwnedAcquisition,
    row: &OwnedPerformance,
) -> Result<String, RetainedResultError> {
    let expected = serde_json::to_value(row)?;
    let prefix = format!(
        "row/{}/{}/{}",
        acquisition.reference().meet_id(),
        acquisition.capture().content_digest,
        row.locator,
    );
    let key = format!("{prefix}/{}", serialized_digest(&expected)?);
    match read(view, &key)? {
        Some(actual) => compare(actual, &expected, key),
        None => match read(view, &prefix)? {
            Some(actual) => compare(actual, &expected, prefix),
            None => Err(RetainedResultError::MissingInterpretation { key }),
        },
    }
}

fn read(view: &StoreSnapshot<'_>, key: &str) -> Result<Option<Value>, RetainedResultError> {
    let budget = JournalReadBudget::new(MAX_INTERPRETATION_BYTES)?;
    Ok(view.journal_payload_bounded(OWNED_MEET_PHASE, key, budget)?)
}

fn compare(actual: Value, expected: &Value, key: String) -> Result<String, RetainedResultError> {
    if &actual != expected {
        return Err(RetainedResultError::ChangedInterpretation { key });
    }
    Ok(key)
}
