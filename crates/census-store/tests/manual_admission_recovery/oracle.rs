use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::fixture::{Fixture, Mode};
use super::physical::{Rows, State};
use super::TestResult;

#[path = "accounting.rs"]
mod accounting;

const HEADER: &[u8] = b"manual-context-admission-v1\0header";

pub(super) fn interrupted(
    fixture: &Fixture,
    state: &State,
    marker_digest: &str,
) -> TestResult<Vec<u8>> {
    unchanged_authority(&fixture.before, state)?;
    let before = fixture.before.rows("journal")?;
    let journal = state.rows("journal")?;
    accounting::verify(before, 1)?;
    accounting::verify(journal, 2)?;
    let reservation = reservation_key(fixture.mode.operation());
    check!(!before.contains_key(&reservation));
    let raw = journal.get(&reservation).ok_or("new reservation")?;
    let row: Value = serde_json::from_slice(raw)?;
    check!(eq; row["decision"]["kind"].as_str(), Some(fixture.mode.kind()));
    let record = original(fixture.mode, &row["decision"]["record"])?;
    check!(raw
        .windows(record.len())
        .any(|part| part == record.as_slice()));
    check!(eq; row["decision"]["record"]["affected_context"].as_str(), Some(fixture.context_root.as_str()));
    check!(eq; row["charges"][0]["roots"].as_u64(), Some(0));
    check!(eq; row["charges"][0]["bytes"].as_u64(), Some(0));
    check!(eq; row["charges"][1]["roots"].as_u64(), Some(1));
    let chunk_key = key("chunk", &fixture.context_root, Some(0));
    check!(!before.contains_key(&chunk_key));
    let chunk = journal.get(&chunk_key).ok_or("durable partial chunk")?;
    check!(eq; digest(chunk), marker_digest);
    let decoded = chunk_bytes(&fixture.context_root, chunk)?;
    affected(fixture, &row["decision"]["record"], &decoded)?;
    check!(!journal.contains_key(&key("index", &fixture.context_root, Some(0))));
    check!(!journal.contains_key(&key("manifest", &fixture.context_root, None)));
    let mut expected = before.clone();
    check!(expected
        .insert(
            HEADER.to_vec(),
            journal.get(HEADER).ok_or("header")?.clone()
        )
        .is_some());
    check!(expected.insert(reservation, raw.clone()).is_none());
    check!(expected.insert(chunk_key, chunk.clone()).is_none());
    check!(eq; &expected, journal);
    Ok(record)
}

pub(super) fn recovered(fixture: &Fixture, interrupted: &State, recovered: &State) -> TestResult {
    unchanged_authority(interrupted, recovered)?;
    let before = interrupted.rows("journal")?;
    let after = recovered.rows("journal")?;
    accounting::verify(after, 2)?;
    let root = &fixture.context_root;
    let chunk = before
        .get(&key("chunk", root, Some(0)))
        .ok_or("partial chunk")?;
    let bytes = chunk_bytes(root, chunk)?;
    let records = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .collect::<Vec<_>>();
    let mut offset = 0usize;
    let entries = records
        .iter()
        .enumerate()
        .map(|(sequence, record)| {
            let entry = json!({"sequence":sequence,"offset":offset,"length":record.len()});
            offset = offset.checked_add(record.len()).ok_or("index offset")?;
            Ok(entry)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let index = serde_json::to_vec(&json!({
        "type":"journal-index","version":2,"object":root,"page":0,"entries":entries,
    }))?;
    let manifest = serde_json::to_vec(&json!({
        "type":"journal-manifest","version":2,"object":root,"codec":"jsonl",
        "total_bytes":bytes.len(),"chunks":1,"records":records.len(),"index_pages":1,
        "chunk_digests":[root],
    }))?;
    let mut expected = before.clone();
    include_exact_json(after, &mut expected, key("index", root, Some(0)), &index)?;
    include_exact_json(after, &mut expected, key("manifest", root, None), &manifest)?;
    check!(eq; &expected, after);
    Ok(())
}

fn original(mode: Mode, value: &Value) -> TestResult<Vec<u8>> {
    match mode {
        Mode::Split => Ok(serde_json::to_vec(&serde_json::from_value::<
            LocalIdentityAdjudicationRecordV2,
        >(value.clone())?)?),
        Mode::Withhold => Ok(serde_json::to_vec(&serde_json::from_value::<
            LocalMembershipWithholdingRecordV1,
        >(value.clone())?)?),
    }
}

fn unchanged_authority(before: &State, after: &State) -> TestResult {
    for name in ["entities", "meta", "receipts"] {
        check!(eq; before.rows(name)?, after.rows(name)?);
    }
    Ok(())
}

fn chunk_bytes(root: &str, raw: &[u8]) -> TestResult<Vec<u8>> {
    let chunk: Value = serde_json::from_slice(raw)?;
    check!(eq; chunk["type"].as_str(), Some("journal-chunk"));
    check!(eq; chunk["version"].as_u64(), Some(2));
    check!(eq; chunk["object"].as_str(), Some(root));
    check!(eq; chunk["index"].as_u64(), Some(0));
    let bytes = BASE64_STANDARD.decode(chunk["data"].as_str().ok_or("chunk data")?)?;
    check!(!bytes.is_empty() && bytes.len() <= 16_384);
    check!(eq; chunk["length"].as_u64(), Some(u64::try_from(bytes.len())?));
    check!(eq; chunk["digest"].as_str(), Some(root));
    check!(eq; digest(&bytes), root);
    Ok(bytes)
}

fn affected(fixture: &Fixture, record: &Value, bytes: &[u8]) -> TestResult {
    let entities = fixture.before.rows("entities")?;
    let performances = physical_records(entities, b"performances\0", "performance")?;
    let relays = physical_records(entities, b"relay_results\0", "relay")?;
    check!(eq; performances.len(), 3);
    check!(eq; relays.len(), 3);
    let header = json!({
        "schema":"local-identity-affected-records/v1", "store":fixture.identity,
        "authority_context":record["authority_context"],
        "evidence_generation":fixture.evidence, "derived_generation":fixture.derived,
        "snapshot_sequence":record["snapshot_sequence"], "counts":[performances.len(),relays.len()],
    });
    let expected = std::iter::once(entry("header", "header", header))
        .chain(performances)
        .chain(relays)
        .collect::<Vec<_>>();
    let observed = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .map(serde_json::from_slice::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; expected, observed);
    Ok(())
}

fn physical_records(rows: &Rows, prefix: &[u8], kind: &str) -> TestResult<Vec<Value>> {
    rows.iter().filter(|(key, _)| key.starts_with(prefix)).enumerate()
        .map(|(index, (key, raw))| Ok(entry(&format!("{kind}:{index:020}"), kind, json!({
            "physical_key_base64":BASE64_STANDARD.encode(key), "raw_json":std::str::from_utf8(raw)?,
        })))).collect()
}

fn entry(key: &str, kind: &str, value: Value) -> Value {
    json!({"key":key,"at":"","payload":{"kind":kind,"value":value}})
}

fn include_exact_json(after: &Rows, expected: &mut Rows, key: Vec<u8>, value: &[u8]) -> TestResult {
    let actual = after.get(&key).ok_or("completed physical row")?;
    check!(eq; serde_json::from_slice::<Value>(actual)?, serde_json::from_slice::<Value>(value)?);
    check!(expected.insert(key, actual.clone()).is_none());
    Ok(())
}

fn key(segment: &str, root: &str, ordinal: Option<u32>) -> Vec<u8> {
    let mut key = format!("j2\0{segment}\0{root}").into_bytes();
    if let Some(ordinal) = ordinal {
        key.push(0);
        key.extend_from_slice(&ordinal.to_be_bytes());
    }
    key
}

fn reservation_key(operation: &str) -> Vec<u8> {
    format!(
        "manual-context-admission-v1\0reservation\0{}",
        digest(operation.as_bytes())
    )
    .into_bytes()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
