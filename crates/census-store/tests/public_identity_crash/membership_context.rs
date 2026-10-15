use super::{Expected, Fixture, Rows, State, TestResult};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use census_domain::model::StoreIdentity;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Serialize)]
struct Header<'a> {
    version: u32,
    authority_policy: u32,
    enrichment_policy: u32,
    lineage: &'a StoreIdentity,
    counts: [usize; 4],
}
#[derive(Serialize)]
struct Entry<'a, T> {
    key: &'a str,
    at: &'static str,
    payload: Payload<'a, T>,
}
#[derive(Serialize)]
struct Payload<'a, T> {
    kind: &'a str,
    value: &'a T,
}

pub(super) fn extend(fixture: &Fixture, state: &State) -> TestResult<(String, Rows)> {
    let records = records(&fixture.expected)?;
    let bytes = records.concat();
    check!(bytes.len() <= 16_384);
    let root = format!("{:x}", Sha256::digest(&bytes));
    let mut offset = 0usize;
    let entries = records
        .iter()
        .enumerate()
        .map(|(sequence, row)| {
            let entry = json!({"sequence": sequence, "offset": offset, "length": row.len()});
            offset = offset
                .checked_add(row.len())
                .ok_or("context offset overflow")?;
            Ok(entry)
        })
        .collect::<TestResult<Vec<Value>>>()?;
    let mut expected = fixture.before.rows("journal")?.clone();
    for (segment, ordinal, value) in [
        (
            "chunk",
            Some(0_u32),
            json!({
                "type": "journal-chunk", "version": 2, "object": root, "index": 0,
                "length": bytes.len(), "digest": root, "data": BASE64_STANDARD.encode(&bytes),
            }),
        ),
        (
            "index",
            Some(0),
            json!({
                "type": "journal-index", "version": 2, "object": root, "page": 0, "entries": entries,
            }),
        ),
        (
            "manifest",
            None,
            json!({
                "type": "journal-manifest", "version": 2, "object": root, "codec": "jsonl",
                "total_bytes": bytes.len(), "chunks": 1, "records": records.len(),
                "index_pages": 1, "chunk_digests": [root],
            }),
        ),
    ] {
        let mut key = format!("j2\0{segment}\0{root}").into_bytes();
        if let Some(ordinal) = ordinal {
            key.push(0);
            key.extend_from_slice(&ordinal.to_be_bytes());
        }
        let raw = state
            .rows("journal")?
            .get(&key)
            .ok_or("authority context row")?;
        check!(eq; serde_json::from_slice::<Value>(raw)?, value);
        if let Some(previous) = expected.insert(key, raw.clone()) {
            check!(eq; previous, *raw);
        }
    }
    Ok((root, expected))
}

fn records(expected: &Expected) -> TestResult<Vec<Vec<u8>>> {
    let mut records = vec![record(
        "header",
        "header",
        &Header {
            version: 1,
            authority_policy: 1,
            enrichment_policy: 1,
            lineage: &expected.identity,
            counts: [2, 0, 1, 1],
        },
    )?];
    for (index, athlete) in expected.athletes.iter().enumerate() {
        records.push(record(&format!("athlete:{index:020}"), "athlete", athlete)?);
    }
    records.push(record(
        "case:00000000000000000000",
        "case",
        expected.cases.first().ok_or("case")?,
    )?);
    records.push(record(
        "verdict:00000000000000000000",
        "verdict",
        expected.verdicts.first().ok_or("verdict")?,
    )?);
    Ok(records)
}

fn record<T: Serialize>(key: &str, kind: &str, value: &T) -> TestResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec(&Entry {
        key,
        at: "",
        payload: Payload { kind, value },
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}
