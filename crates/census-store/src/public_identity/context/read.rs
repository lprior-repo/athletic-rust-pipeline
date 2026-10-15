use super::Inputs;
#[cfg(test)]
use crate::StoreError;
use crate::StoreResult;
use census_domain::model::StoreIdentity;
#[cfg(test)]
use census_domain::model::{AppliedAthleteIdentity, SourceIdentity};
use fjall::{Keyspace, Snapshot};

pub(in crate::public_identity) fn read(
    snapshot: &Snapshot,
    journal: &Keyspace,
    root: &str,
    identity: &StoreIdentity,
) -> StoreResult<Inputs> {
    let object = crate::read::read_context_object(snapshot, journal, root)?;
    let records = object.records().map(|record| {
        let record = record?;
        Ok((record.length(), record.entry().clone()))
    });
    super::decode::from_records(records, object.record_count(), identity)
}

#[cfg(test)]
pub(in crate::public_identity) fn verify(
    snapshot: &Snapshot,
    journal: &Keyspace,
    root: &str,
    decision: &AppliedAthleteIdentity,
) -> StoreResult<SourceIdentity> {
    let identity = match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity) => identity,
        _ => return Err(invalid("retained authority lineage missing")),
    };
    let inputs = read(snapshot, journal, root, &identity)?;
    super::verify::source(&inputs, decision)
}

#[cfg(test)]
pub(in crate::public_identity) fn verify_reviewed(
    snapshot: &Snapshot,
    journal: &Keyspace,
    root: &str,
    decision: &AppliedAthleteIdentity,
) -> StoreResult<()> {
    let identity = match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity) => identity,
        _ => return Err(invalid("retained authority lineage missing")),
    };
    let inputs = read(snapshot, journal, root, &identity)?;
    super::verify::reviewed(&inputs, decision)
}

#[cfg(test)]
fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}
