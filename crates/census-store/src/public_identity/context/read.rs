use super::storage::charge_section_bytes;
use super::{wire, Inputs, BUDGET};
use crate::{StoreError, StoreResult};
use census_domain::model::{AppliedAthleteIdentity, SourceIdentity, StoreIdentity};
use fjall::{Keyspace, Snapshot};
use serde::de::DeserializeOwned;

pub(super) fn read(
    snapshot: &Snapshot,
    journal: &Keyspace,
    root: &str,
    identity: &StoreIdentity,
) -> StoreResult<Inputs> {
    let object = crate::read::read_context_object(snapshot, journal, root)?;
    let mut records = object.records();
    let header: wire::Header = value(next_record(&mut records)?, "header", "header")?;
    if header.version != 1
        || header.authority_policy != 1
        || header.enrichment_policy != 1
        || &header.lineage != identity
    {
        return Err(invalid("unsupported or foreign retained authority context"));
    }
    let total = header.counts.iter().try_fold(1usize, |total, count| {
        if u64::try_from(*count).map_err(|_| StoreError::CounterOverflow)? > BUDGET.rows {
            return Err(invalid("retained authority section exceeds its row bound"));
        }
        total.checked_add(*count).ok_or(StoreError::CounterOverflow)
    })?;
    if total != object.record_count() {
        return Err(invalid("retained authority context count mismatch"));
    }
    let [athletes, observations, cases, verdicts] = header.counts;
    let inputs = Inputs {
        athletes: rows(&mut records, "athlete", athletes)?,
        observations: rows(&mut records, "observation", observations)?,
        cases: rows(&mut records, "case", cases)?,
        verdicts: rows(&mut records, "verdict", verdicts)?,
    };
    match records.next() {
        Some(Err(error)) => return Err(error),
        Some(Ok(_)) => return Err(invalid("extra retained authority records")),
        None => {}
    }
    Ok(inputs)
}

fn rows<T: DeserializeOwned>(
    records: &mut crate::JournalRecords<'_, '_>,
    kind: &str,
    count: usize,
) -> StoreResult<Vec<T>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|error| StoreError::Refused {
            detail: format!("retained authority allocation: {error}"),
        })?;
    (0..count).try_fold(0u64, |bytes, index| {
        let record = next_record(records)?;
        let bytes = charge_section_bytes(bytes, record.length())?;
        rows.push(value(record, &format!("{kind}:{index:020}"), kind)?);
        Ok::<_, StoreError>(bytes)
    })?;
    Ok(rows)
}

fn next_record(records: &mut crate::JournalRecords<'_, '_>) -> StoreResult<crate::JournalRecord> {
    records
        .next()
        .ok_or_else(|| invalid("retained authority record missing"))?
}

fn value<T: DeserializeOwned>(
    record: crate::JournalRecord,
    key: &str,
    kind: &str,
) -> StoreResult<T> {
    if record.entry().key != key || !record.entry().at.is_empty() {
        return Err(invalid("retained authority record envelope mismatch"));
    }
    let decoded: wire::Decoded<T> =
        serde_json::from_value(record.payload().clone()).map_err(|source| StoreError::Decode {
            key: key.into(),
            source,
        })?;
    if decoded.kind != kind {
        return Err(invalid("retained authority section order mismatch"));
    }
    Ok(decoded.value)
}

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
    let [member] = decision.members.as_slice() else {
        return Err(invalid("unsupported retained authority membership"));
    };
    let inputs = read(snapshot, journal, root, &identity)?;
    let (accepted, source) =
        super::super::authority::accept_inputs(&inputs, &member.subject, &decision.observed_at)?;
    if accepted.record() != decision {
        return Err(invalid("retained authority decision does not replay"));
    }
    Ok(source)
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

#[cfg(test)]
#[path = "section_budget_tests.rs"]
mod section_budget_tests;
