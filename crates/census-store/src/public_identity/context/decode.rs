use super::storage::charge_section_bytes;
use super::{wire, Inputs, BUDGET};
use crate::{StoreError, StoreResult};
use census_domain::model::StoreIdentity;
use serde::de::DeserializeOwned;
#[cfg(test)]
use sha2::{Digest, Sha256};

type Record = (usize, crate::read::JournalEntry);

#[cfg(test)]
pub(in crate::public_identity) fn from_bytes(
    bytes: &[u8],
    root: &str,
    identity: &StoreIdentity,
) -> StoreResult<Inputs> {
    if bytes.len() > crate::journal::context::CONTEXT_TOTAL_BYTES {
        return Err(invalid("authority context exceeds retained input limits"));
    }
    if !crate::journal::is_object_id(root) || format!("{:x}", Sha256::digest(bytes)) != root {
        return Err(invalid("frozen authority context digest mismatch"));
    }
    let count = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .take(crate::journal::context::CONTEXT_RECORDS + 1)
        .count();
    if count > crate::journal::context::CONTEXT_RECORDS {
        return Err(invalid("authority context exceeds retained input limits"));
    }
    let records = bytes.split_inclusive(|byte| *byte == b'\n').map(|line| {
        if line.last() != Some(&b'\n') || line.len() > crate::journal::context::CONTEXT_RECORD_BYTES
        {
            return Err(invalid("invalid retained authority record bytes"));
        }
        Ok((
            line.len(),
            super::super::codec::decode("frozen authority record", line)?,
        ))
    });
    from_records(records, count, identity)
}

pub(super) fn from_records(
    mut records: impl Iterator<Item = StoreResult<Record>>,
    count: usize,
    identity: &StoreIdentity,
) -> StoreResult<Inputs> {
    let header: wire::Header = value(next(&mut records)?.1, "header", "header")?;
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
    if total != count {
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
    records: &mut impl Iterator<Item = StoreResult<Record>>,
    kind: &str,
    count: usize,
) -> StoreResult<Vec<T>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|error| StoreError::Refused {
            detail: format!("retained authority allocation: {error}"),
        })?;
    (0..count).try_fold(0u64, |bytes, index| {
        let (length, record) = next(records)?;
        let bytes = charge_section_bytes(bytes, length)?;
        rows.push(value(record, &format!("{kind}:{index:020}"), kind)?);
        Ok::<_, StoreError>(bytes)
    })?;
    Ok(rows)
}

fn next(records: &mut impl Iterator<Item = StoreResult<Record>>) -> StoreResult<Record> {
    records
        .next()
        .ok_or_else(|| invalid("retained authority record missing"))?
}

fn value<T: DeserializeOwned>(
    record: crate::read::JournalEntry,
    key: &str,
    kind: &str,
) -> StoreResult<T> {
    if record.key != key || !record.at.is_empty() {
        return Err(invalid("retained authority record envelope mismatch"));
    }
    let decoded: wire::Decoded<T> =
        serde_json::from_value(record.payload).map_err(|source| StoreError::Decode {
            key: key.into(),
            source,
        })?;
    if decoded.kind != kind {
        return Err(invalid("retained authority section order mismatch"));
    }
    Ok(decoded.value)
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}
