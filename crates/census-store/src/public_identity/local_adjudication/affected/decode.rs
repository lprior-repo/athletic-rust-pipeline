use super::{
    allocation, invalid, Header, Inputs, Physical, MAX_KEY_BASE64_BYTES, MAX_KEY_BYTES, SCHEMA,
};
use crate::public_identity::context::{storage::charge_section_bytes, wire, BUDGET};
use crate::{Entity, StoreError, StoreResult, Table};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::model::{CanonicalPerformance, RelayResult};
use serde::de::DeserializeOwned;
#[cfg(test)]
use sha2::{Digest, Sha256};

type Record = (usize, crate::read::JournalEntry);

pub(in crate::public_identity) fn read(
    snapshot: &fjall::Snapshot,
    journal: &fjall::Keyspace,
    root: &str,
) -> StoreResult<Inputs> {
    let object = crate::read::read_context_object(snapshot, journal, root)?;
    let records = object.records().map(|record| {
        let record = record?;
        Ok((record.length(), record.entry().clone()))
    });
    from_records(records, object.record_count())
}

#[cfg(test)]
pub(in crate::public_identity) fn from_bytes(bytes: &[u8], root: &str) -> StoreResult<Inputs> {
    if bytes.len() > crate::journal::context::CONTEXT_TOTAL_BYTES
        || !crate::journal::is_object_id(root)
        || format!("{:x}", Sha256::digest(bytes)) != root
    {
        return Err(invalid("manual affected context digest or byte bound"));
    }
    let count = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .take(crate::journal::context::CONTEXT_RECORDS + 1)
        .count();
    if count > crate::journal::context::CONTEXT_RECORDS {
        return Err(invalid("manual affected inventory count"));
    }
    let records = bytes.split_inclusive(|byte| *byte == b'\n').map(|line| {
        if line.last() != Some(&b'\n') || line.len() > crate::journal::context::CONTEXT_RECORD_BYTES
        {
            return Err(invalid("manual affected retained record bound"));
        }
        Ok((
            line.len(),
            crate::public_identity::codec::decode("manual affected record", line)?,
        ))
    });
    from_records(records, count)
}

fn from_records(
    mut records: impl Iterator<Item = StoreResult<Record>>,
    count: usize,
) -> StoreResult<Inputs> {
    let header: Header = value(next(&mut records)?.1, "header", "header")?;
    if header.schema != SCHEMA || !crate::journal::is_object_id(&header.authority_context) {
        return Err(invalid("manual affected inventory schema"));
    }
    let total = header.counts.iter().try_fold(1usize, |total, count| {
        if u64::try_from(*count).map_err(|_| StoreError::CounterOverflow)? > BUDGET.rows {
            return Err(invalid("manual affected section row bound"));
        }
        total.checked_add(*count).ok_or(StoreError::CounterOverflow)
    })?;
    if total != count || count > crate::journal::context::CONTEXT_RECORDS {
        return Err(invalid("manual affected inventory count"));
    }
    let [performances, relays] = header.counts;
    let inputs = Inputs {
        header,
        performances: rows::<CanonicalPerformance>(
            &mut records,
            "performance",
            Table::Performances,
            performances,
        )?,
        relays: rows::<RelayResult>(&mut records, "relay", Table::RelayResults, relays)?,
    };
    match records.next() {
        Some(Err(error)) => return Err(error),
        Some(Ok(_)) => return Err(invalid("extra manual affected records")),
        None => {}
    }
    Ok(inputs)
}

fn rows<T: Entity>(
    records: &mut impl Iterator<Item = StoreResult<Record>>,
    kind: &str,
    table: Table,
    count: usize,
) -> StoreResult<Vec<Physical>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(allocation)?;
    let mut previous = Vec::new();
    (0..count).try_fold(0u64, |bytes, index| {
        let (length, record) = next(records)?;
        let bytes = charge_section_bytes(bytes, length)?;
        let row: Physical = value(record, &format!("{kind}:{index:020}"), kind)?;
        let key = row.key()?;
        if key <= previous {
            return Err(invalid("manual affected physical order"));
        }
        row.decode::<T>(table, &key)?;
        previous = key;
        rows.push(row);
        Ok::<_, StoreError>(bytes)
    })?;
    Ok(rows)
}

impl Physical {
    pub(super) fn key(&self) -> StoreResult<Vec<u8>> {
        if self.physical_key_base64.len() > MAX_KEY_BASE64_BYTES
            || self.raw_json.len() > BUDGET.record_bytes
        {
            return Err(invalid("manual affected physical byte bound"));
        }
        let key = STANDARD
            .decode(&self.physical_key_base64)
            .map_err(|_| invalid("manual affected physical key encoding"))?;
        if key.len() > MAX_KEY_BYTES || STANDARD.encode(&key) != self.physical_key_base64 {
            return Err(invalid("manual affected physical key bound"));
        }
        Ok(key)
    }

    pub(super) fn decode<T: Entity>(&self, table: Table, key: &[u8]) -> StoreResult<T> {
        crate::read::decode_physical_observation(table, key, self.raw_json.as_bytes())
    }
}

fn next(records: &mut impl Iterator<Item = StoreResult<Record>>) -> StoreResult<Record> {
    records
        .next()
        .ok_or_else(|| invalid("manual affected record missing"))?
}

fn value<T: DeserializeOwned>(
    record: crate::read::JournalEntry,
    key: &str,
    kind: &str,
) -> StoreResult<T> {
    if record.key != key || !record.at.is_empty() {
        return Err(invalid("manual affected record envelope"));
    }
    let decoded: wire::Decoded<T> =
        serde_json::from_value(record.payload).map_err(|source| StoreError::Decode {
            key: key.into(),
            source,
        })?;
    if decoded.kind != kind {
        return Err(invalid("manual affected section order"));
    }
    Ok(decoded.value)
}
