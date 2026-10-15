use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::fixture::Fixture;
use super::physical::{Rows, State};
use super::TestResult;

#[derive(Serialize)]
struct Header<'a> {
    schema: &'static str,
    store: &'a StoreIdentity,
    authority_context: &'a str,
    evidence_generation: u64,
    derived_generation: u64,
    snapshot_sequence: u64,
    counts: [usize; 2],
}

#[derive(Serialize)]
struct Physical<'a> {
    physical_key_base64: String,
    raw_json: &'a str,
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

pub(super) fn extend(
    fixture: &Fixture,
    state: &State,
    record: &LocalIdentityAdjudicationRecordV2,
    before: &Rows,
) -> TestResult<Rows> {
    let records = records(fixture, record)?;
    let bytes = records.concat();
    check!(
        bytes.len() <= 16_384,
        "fixture context must fit one bounded chunk"
    );
    let root = digest(&bytes);
    check!(eq; root, record.affected_context);
    let mut expected = before.clone();
    let journal = state.rows("journal")?;
    let chunk = include(journal, &mut expected, key("chunk", &root, Some(0)))?;
    check!(eq; chunk, json!({
        "type": "journal-chunk", "version": 2, "object": root,
        "index": 0, "length": bytes.len(), "digest": root,
        "data": BASE64_STANDARD.encode(&bytes),
    }));
    let index = include(journal, &mut expected, key("index", &root, Some(0)))?;
    check!(eq; index, json!({
        "type": "journal-index", "version": 2, "object": root, "page": 0,
        "entries": indices(&records)?,
    }));
    let manifest = include(journal, &mut expected, key("manifest", &root, None))?;
    check!(eq; manifest, json!({
        "type": "journal-manifest", "version": 2, "object": root,
        "codec": "jsonl", "total_bytes": bytes.len(), "chunks": 1,
        "records": records.len(), "index_pages": 1, "chunk_digests": [root],
    }));
    Ok(expected)
}

fn records(
    fixture: &Fixture,
    prepared: &LocalIdentityAdjudicationRecordV2,
) -> TestResult<Vec<Vec<u8>>> {
    let entities = fixture.before.rows("entities")?;
    let performances = physical(entities, b"performances\0", "performance")?;
    let relays = physical(entities, b"relay_results\0", "relay")?;
    check!(eq; performances.len(), 3);
    check!(eq; relays.len(), 3);
    let header = record(
        "header",
        "header",
        &Header {
            schema: "local-identity-affected-records/v1",
            store: &fixture.plan.identity,
            authority_context: &prepared.authority_context,
            evidence_generation: fixture.evidence,
            derived_generation: fixture.derived,
            snapshot_sequence: prepared.snapshot_sequence,
            counts: [performances.len(), relays.len()],
        },
    )?;
    Ok(std::iter::once(header)
        .chain(performances)
        .chain(relays)
        .collect())
}

fn physical(entities: &Rows, prefix: &[u8], kind: &str) -> TestResult<Vec<Vec<u8>>> {
    entities
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
        .enumerate()
        .map(|(index, (key, raw))| {
            record(
                &format!("{kind}:{index:020}"),
                kind,
                &Physical {
                    physical_key_base64: BASE64_STANDARD.encode(key),
                    raw_json: std::str::from_utf8(raw)?,
                },
            )
        })
        .collect()
}

fn indices(records: &[Vec<u8>]) -> TestResult<Vec<Value>> {
    let mut offset = 0usize;
    records
        .iter()
        .enumerate()
        .map(|(sequence, record)| {
            let value = json!({"sequence": sequence, "offset": offset, "length": record.len()});
            offset = offset
                .checked_add(record.len())
                .ok_or("fixture index offset overflow")?;
            Ok(value)
        })
        .collect()
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

fn include(journal: &Rows, expected: &mut Rows, key: Vec<u8>) -> TestResult<Value> {
    let raw = journal
        .get(&key)
        .ok_or("retained affected context row missing")?;
    if let Some(previous) = expected.insert(key, raw.clone()) {
        check!(eq; previous, *raw);
    }
    Ok(serde_json::from_slice(raw)?)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
