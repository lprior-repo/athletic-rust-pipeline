use census_domain::model::{serialized_digest, StoreIdentity};
use census_store::GraphAuthorityReferenceV1;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::physical::{Rows, State};
use super::TestResult;

#[path = "graph_accounting.rs"]
mod accounting;
#[path = "graph_physical.rs"]
mod physical;

pub(crate) const HEADER: &[u8] = b"manual-context-admission-v1\0header";
pub(crate) const PREFIX: &[u8] = b"manual-context-admission-v1\0reservation\0";

pub(crate) struct Prepared {
    pub graph: GraphAuthorityReferenceV1,
    pub reservation: Value,
    pub journal: Rows,
}

pub(crate) fn prepared(
    before: &State,
    state: &State,
    identity: &StoreIdentity,
    operation: &str,
) -> TestResult<Prepared> {
    let journal = state.rows("journal")?;
    verify_accounting(journal)?;
    let reservation = row(journal, &reservation_key(operation))?;
    check!(eq; reservation["version"], json!(4));
    check!(eq; reservation["store"], json!(identity));
    check!(eq; reservation["operation"], json!(operation));
    let raw_operation = raw_operation(identity, operation)?;
    check!(eq; reservation["raw_operation"], json!(raw_operation));
    let graph: GraphAuthorityReferenceV1 = serde_json::from_value(reservation["graph"].clone())?;
    check!(eq; reservation["graph"]["capture"]["store"], json!(identity));
    check!(eq; reservation["graph"]["capture"]["format"]["lifecycle_revision"], json!(7));
    check!(eq; reservation["evidence_generation"], reservation["graph"]["capture"]["evidence_generation"]);
    check!(eq; reservation["derived_generation"], reservation["graph"]["capture"]["derived_generation"]);
    check!(eq; reservation["witness"]["graph"], reservation["graph"]);
    check!(eq; reservation["witness"]["operation"], json!(operation));
    check!(eq; reservation["witness"]["observed_at"], reservation["observed_at"]);
    let mut expected = extend_raw(before, state, identity, operation, true)?;
    include(journal, &mut expected, reservation_key(operation))?;
    let raw = row(journal, &reservation_key(&raw_operation))?;
    check!(eq; raw["purpose"]["capture"], reservation["graph"]["capture"]);
    check!(eq; raw["purpose"]["seal"], reservation["graph"]["seal"]);
    Ok(Prepared {
        graph,
        reservation,
        journal: expected,
    })
}

pub(crate) fn extend_raw(
    before: &State,
    state: &State,
    identity: &StoreIdentity,
    operation: &str,
    complete: bool,
) -> TestResult<Rows> {
    let journal = state.rows("journal")?;
    let key = reservation_key(&raw_operation(identity, operation)?);
    let reservation = row(journal, &key)?;
    let new_reservation = !before.rows("journal")?.contains_key(&key);
    check!(eq; reservation["version"], json!(3));
    check!(eq; reservation["purpose"]["kind"], json!("raw_generation"));
    check!(eq; reservation["purpose"]["operation"], json!(raw_operation(identity, operation)?));
    check!(eq; reservation["purpose"]["capture"]["store"], json!(identity));
    let mut expected = before.rows("journal")?.clone();
    include(journal, &mut expected, key)?;
    expected.insert(
        HEADER.to_vec(),
        journal.get(HEADER).ok_or("admission header")?.clone(),
    );
    for descriptor in reservation["descriptors"]
        .as_array()
        .ok_or("graph descriptors")?
    {
        check!(eq; descriptor["incremental"]["root"], descriptor["object"]["root"]);
        if new_reservation {
            let root = descriptor["object"]["root"].as_str().ok_or("object root")?;
            let ready = before
                .rows("journal")?
                .contains_key(&self::key("manifest", root, None)?);
            check!(eq; descriptor["incremental"]["roots"], json!(u64::from(!ready)));
            let bytes = if ready {
                json!(0)
            } else {
                descriptor["physical"]["full_bytes"].clone()
            };
            check!(eq; descriptor["incremental"]["bytes"], bytes);
        }
        let rows = physical::verify(journal, descriptor, complete)?;
        physical::verify_source_rows(&rows, before.rows("entities")?)?;
        for (key, value) in rows {
            if let Some(prior) = expected.insert(key, value.clone()) {
                check!(eq; prior, value);
            }
        }
    }
    Ok(expected)
}

pub(crate) fn verify_accounting(journal: &Rows) -> TestResult {
    accounting::verify(journal)
}

pub(crate) fn reservation_key(operation: &str) -> Vec<u8> {
    [PREFIX, digest(operation.as_bytes()).as_bytes()].concat()
}

pub(crate) fn raw_operation(identity: &StoreIdentity, operation: &str) -> TestResult<String> {
    Ok(format!(
        "identity-authority-{}",
        serialized_digest(&(
            "public-identity-raw-operation/v1",
            identity,
            operation,
            "identity-authority",
        ))?
    ))
}

pub(crate) fn row(journal: &Rows, key: &[u8]) -> TestResult<Value> {
    Ok(serde_json::from_slice(
        journal.get(key).ok_or("retained graph row")?,
    )?)
}

pub(crate) fn key(segment: &str, root: &str, ordinal: Option<usize>) -> TestResult<Vec<u8>> {
    let mut key = format!("j2\0{segment}\0{root}").into_bytes();
    if let Some(ordinal) = ordinal {
        key.push(0);
        key.extend_from_slice(&u32::try_from(ordinal)?.to_be_bytes());
    }
    Ok(key)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn include(journal: &Rows, expected: &mut Rows, key: Vec<u8>) -> TestResult {
    let raw = journal.get(&key).ok_or("expected graph physical row")?;
    if let Some(previous) = expected.insert(key, raw.clone()) {
        check!(eq; previous, *raw);
    }
    Ok(())
}
