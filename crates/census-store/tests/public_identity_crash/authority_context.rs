use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use census_domain::model::{CanonicalAthlete, StoreIdentity};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{Fixture, Rows, State, TestResult};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RetainedContext {
    pub root: String,
    rows: Rows,
}

impl RetainedContext {
    pub fn extend_journal(&self, before: &Rows) -> TestResult<Rows> {
        let mut expected = before.clone();
        for (key, raw) in &self.rows {
            if let Some(existing) = expected.insert(key.clone(), raw.clone()) {
                check!(eq; existing, *raw);
            }
        }
        Ok(expected)
    }
}

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
    at: &'a str,
    payload: Payload<'a, T>,
}

#[derive(Serialize)]
struct Payload<'a, T> {
    kind: &'a str,
    value: &'a T,
}

pub(super) fn read(fixture: &Fixture, state: &State) -> TestResult<RetainedContext> {
    let header = record(
        "header",
        "header",
        &Header {
            version: 1,
            authority_policy: 1,
            enrichment_policy: 1,
            lineage: &fixture.identity,
            counts: [1, 0, 0, 0],
        },
    )?;
    let athlete = record("athlete:00000000000000000000", "athlete", &fixture.athlete)?;
    let expected_bytes = [header.as_slice(), athlete.as_slice()].concat();
    check!(
        expected_bytes.len() <= 16_384,
        "fixture must fit one bounded physical chunk"
    );
    let root = digest(&expected_bytes);
    let mut rows = Rows::new();
    let journal = state.rows("journal")?;
    let chunk = read_row(journal, &mut rows, key("chunk", &root, Some(0)))?;
    let data = chunk
        .get("data")
        .and_then(Value::as_str)
        .ok_or("missing chunk data")?;
    let bytes = BASE64_STANDARD.decode(data)?;
    check!(eq; bytes, expected_bytes);
    check!(eq; digest(&bytes), root);
    check!(eq; chunk, json!({
        "type": "journal-chunk", "version": 2, "object": root,
        "index": 0, "length": bytes.len(), "digest": root,
        "data": BASE64_STANDARD.encode(&expected_bytes),
    }));
    let index = read_row(journal, &mut rows, key("index", &root, Some(0)))?;
    check!(eq; index, json!({
        "type": "journal-index", "version": 2, "object": root, "page": 0,
        "entries": [
            { "sequence": 0, "offset": 0, "length": header.len() },
            { "sequence": 1, "offset": header.len(), "length": athlete.len() },
        ],
    }));
    let manifest = read_row(journal, &mut rows, key("manifest", &root, None))?;
    check!(eq; manifest, json!({
        "type": "journal-manifest", "version": 2, "object": root,
        "codec": "jsonl", "total_bytes": bytes.len(), "chunks": 1,
        "records": 2, "index_pages": 1, "chunk_digests": [root],
    }));
    let retained: Value = serde_json::from_slice(
        bytes
            .get(header.len()..)
            .ok_or("missing retained athlete record")?,
    )?;
    let retained: CanonicalAthlete = serde_json::from_value(
        retained
            .get("payload")
            .and_then(|payload| payload.get("value"))
            .ok_or("missing retained athlete payload")?
            .clone(),
    )?;
    check!(eq; retained, fixture.athlete);
    Ok(RetainedContext { root, rows })
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

fn key(segment: &str, root: &str, ordinal: Option<u32>) -> Vec<u8> {
    let mut key = format!("j2\0{segment}\0{root}").into_bytes();
    if let Some(ordinal) = ordinal {
        key.push(0);
        key.extend_from_slice(&ordinal.to_be_bytes());
    }
    key
}

fn read_row(journal: &Rows, rows: &mut Rows, key: Vec<u8>) -> TestResult<Value> {
    let raw = journal.get(&key).ok_or("retained context row missing")?;
    let value = serde_json::from_slice(raw)?;
    check!(rows.insert(key, raw.clone()).is_none());
    Ok(value)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
