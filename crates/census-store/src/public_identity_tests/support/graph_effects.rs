use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{serialized_digest, StoreIdentity};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{PhysicalState, TestResult};
use crate::public_identity::paged_generation::{ObjectRefV2, VerifiedRawGraph};
use crate::{GraphAuthorityReferenceV1, Store};

type Rows = BTreeMap<Vec<u8>, Vec<u8>>;
const HEADER: &[u8] = b"manual-context-admission-v1\0header";
const PREFIX: &[u8] = b"manual-context-admission-v1\0reservation\0";

#[path = "../../../tests/public_identity_crash/graph_accounting.rs"]
mod accounting;
#[path = "../../../tests/public_identity_crash/graph_physical.rs"]
mod physical;
#[path = "../../../tests/public_identity_crash/graph_source.rs"]
mod source;

pub(crate) fn assert_graph_staging(
    store: &Store,
    before: &PhysicalState,
    after: &PhysicalState,
    identity: &StoreIdentity,
    operation: &str,
) -> TestResult {
    assert_graph_staging_bounded(store, before, after, identity, operation, 2_048)
}

pub(crate) fn assert_graph_staging_bounded(
    store: &Store,
    before: &PhysicalState,
    after: &PhysicalState,
    identity: &StoreIdentity,
    operation: &str,
    maximum_rows: usize,
) -> TestResult {
    check!(eq; before.len(), 4);
    check!(eq; after.len(), 4);
    for name in [crate::ENTITIES, crate::META, crate::RECEIPTS] {
        check!(eq; rows(after, name)?, rows(before, name)?);
    }
    let journal = rows(after, crate::JOURNAL)?;
    let expected = staged_journal(store, before, &journal, identity, operation, maximum_rows)?;
    check!(eq; journal, expected);
    accounting::verify(&journal)
}

fn staged_journal(
    store: &Store,
    before: &PhysicalState,
    journal: &Rows,
    identity: &StoreIdentity,
    operation: &str,
    maximum_rows: usize,
) -> TestResult<Rows> {
    let mut expected = rows(before, crate::JOURNAL)?;
    let raw_operation = raw_operation(identity, operation)?;
    let reservation_key = reservation_key(&raw_operation);
    check!(!expected.contains_key(&reservation_key));
    let reservation = row(journal, &reservation_key)?;
    check!(eq; reservation["version"], json!(3));
    check!(eq; reservation["purpose"]["kind"], json!("raw_generation"));
    check!(eq; reservation["purpose"]["operation"], json!(raw_operation));
    let entities = rows(before, crate::ENTITIES)?;
    let meta = rows(before, crate::META)?;
    let mut source = source::SourceImage::capture_bounded(&entities, &meta, maximum_rows)?;
    verify_capture(&reservation["purpose"]["capture"], &meta, identity)?;
    check!(eq; reservation["purpose"]["capture"]["derived_generation"], json!(source.derived_generation()));
    verify_header(&expected, journal, &reservation_key, &reservation, identity)?;
    include(journal, &mut expected, reservation_key)?;
    expected.insert(
        HEADER.to_vec(),
        journal.get(HEADER).ok_or("admission header")?.clone(),
    );
    let descriptors = reservation["descriptors"]
        .as_array()
        .ok_or("graph descriptors")?;
    check!(!descriptors.is_empty() && descriptors.len() <= 4096);
    verify_graph(store, &reservation, descriptors)?;
    for descriptor in descriptors {
        extend_object(journal, &mut expected, descriptor, &mut source)?;
    }
    source.finish(true)?;
    Ok(expected)
}

fn extend_object(
    journal: &Rows,
    expected: &mut Rows,
    descriptor: &Value,
    source: &mut source::SourceImage,
) -> TestResult {
    let root = descriptor["object"]["root"].as_str().ok_or("object root")?;
    let ready = expected.contains_key(&key("manifest", root, None)?);
    check!(eq; descriptor["incremental"]["root"], json!(root));
    check!(eq; descriptor["incremental"]["roots"], json!(u64::from(!ready)));
    let bytes = if ready {
        json!(0)
    } else {
        descriptor["physical"]["full_bytes"].clone()
    };
    check!(eq; descriptor["incremental"]["bytes"], bytes);
    let physical = physical::verify(journal, descriptor, true)?;
    source.observe_object(&physical)?;
    for (key, value) in physical {
        if let Some(prior) = expected.insert(key, value.clone()) {
            check!(eq; prior, value);
        }
    }
    Ok(())
}

fn verify_capture(capture: &Value, meta: &Rows, identity: &StoreIdentity) -> TestResult {
    let revisions = row(meta, b"capability:public-identity-input-revisions-v2")?;
    check!(eq; revisions["version"], json!(2));
    let expected = json!({
        "store": identity,
        "format": {"store_schema": 2, "key_format": 1, "lifecycle_revision": 7},
        "run": {"kind": "unbound"},
        "evidence_generation": counter(meta, b"generation:entities")?,
        "derived_generation": counter(meta, b"generation:derived:current")?,
        "snapshot_sequence": capture["snapshot_sequence"].as_u64().ok_or("capture sequence")?,
        "table_revisions": revisions["counters"],
        "authority_policy": 1, "enrichment_policy": 1, "projection_policy": 1,
        "claim_ordinals": "full-section-v1",
    });
    check!(eq; capture, &expected);
    Ok(())
}

fn verify_graph(store: &Store, reservation: &Value, descriptors: &[Value]) -> TestResult {
    let graph: GraphAuthorityReferenceV1 = serde_json::from_value(json!({
        "version": 1,
        "capture": reservation["purpose"]["capture"],
        "seal": reservation["purpose"]["seal"],
    }))?;
    let snapshot = store.snapshot();
    VerifiedRawGraph::verify(&snapshot, graph.capture(), graph.seal())?;
    let seal = snapshot.raw_generation_seal(graph.seal(), graph.capture())?;
    let mut seen = BTreeSet::new();
    let children = seal
        .tables()
        .iter()
        .map(|table| table.object())
        .collect::<Vec<_>>();
    verify_descriptor(descriptors, &mut seen, graph.seal(), &children)?;
    for table_ref in seal.tables() {
        let binding = graph.capture().table_binding(table_ref.table())?;
        let table = snapshot.raw_generation_table(&table_ref.object(), &binding)?;
        let children = table
            .blocks()
            .iter()
            .map(|block| block.object())
            .collect::<Vec<_>>();
        verify_descriptor(descriptors, &mut seen, &table_ref.object(), &children)?;
        for block_ref in table.blocks() {
            let block = snapshot.raw_generation_block(&block_ref.object(), &binding)?;
            let children = block
                .pages()
                .iter()
                .map(|page| page.object())
                .collect::<Vec<_>>();
            verify_descriptor(descriptors, &mut seen, &block_ref.object(), &children)?;
            for page in block.pages() {
                verify_descriptor(descriptors, &mut seen, &page.object(), &[])?;
            }
        }
    }
    check!(eq; seen.len(), descriptors.len());
    Ok(())
}

fn verify_descriptor(
    descriptors: &[Value],
    seen: &mut BTreeSet<String>,
    object: &ObjectRefV2,
    children: &[ObjectRefV2],
) -> TestResult {
    check!(seen.len() < 4096);
    check!(seen.insert(object.root().as_str().to_owned()));
    let found = descriptors
        .iter()
        .filter(|entry| entry["object"]["root"].as_str() == Some(object.root().as_str()))
        .collect::<Vec<_>>();
    let [descriptor] = found.as_slice() else {
        return Err("exact graph descriptor".into());
    };
    check!(eq; descriptor["object"], serde_json::to_value(object)?);
    check!(eq; descriptor["children"], serde_json::to_value(children)?);
    Ok(())
}

fn verify_header(
    before: &Rows,
    after: &Rows,
    reservation_key: &[u8],
    reservation: &Value,
    identity: &StoreIdentity,
) -> TestResult {
    let prior = row(before, HEADER)?;
    let mut expected = json!({
        "version": 2, "policy": "manual-context-admission/v2", "store": identity,
        "bytes": prior["bytes"].as_u64().ok_or("prior bytes")?,
        "roots": prior["roots"].as_u64().ok_or("prior roots")?,
        "reservations": prior["reservations"].as_u64().ok_or("prior reservations")?,
    });
    check!(eq; prior, expected);
    let metadata = reservation_key
        .len()
        .checked_add(after.get(reservation_key).ok_or("reservation bytes")?.len())
        .ok_or("reservation metadata overflow")?;
    add_header(&mut expected, "bytes", u64::try_from(metadata)?)?;
    add_header(&mut expected, "reservations", 1)?;
    for descriptor in reservation["descriptors"].as_array().ok_or("descriptors")? {
        for field in ["bytes", "roots"] {
            add_header(
                &mut expected,
                field,
                descriptor["incremental"][field]
                    .as_u64()
                    .ok_or("incremental charge")?,
            )?;
        }
    }
    check!(eq; row(after, HEADER)?, expected);
    Ok(())
}

fn add_header(header: &mut Value, field: &str, added: u64) -> TestResult {
    let previous = header[field].as_u64().ok_or("header counter")?;
    header[field] = json!(previous
        .checked_add(added)
        .ok_or("header counter overflow")?);
    Ok(())
}

fn rows(state: &PhysicalState, name: &str) -> TestResult<Rows> {
    let mut rows = Rows::new();
    let entries = &state
        .iter()
        .find(|(found, _)| found == name)
        .ok_or("keyspace")?
        .1;
    for (key, value) in entries {
        check!(rows.insert(key.clone(), value.clone()).is_none());
    }
    Ok(rows)
}

fn counter(meta: &Rows, key: &[u8]) -> TestResult<u64> {
    let text = std::str::from_utf8(meta.get(key).ok_or("counter")?)?;
    let value = text.parse::<u64>()?;
    check!(eq; text, value.to_string());
    Ok(value)
}

fn raw_operation(identity: &StoreIdentity, operation: &str) -> TestResult<String> {
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

fn reservation_key(operation: &str) -> Vec<u8> {
    [PREFIX, digest(operation.as_bytes()).as_bytes()].concat()
}

fn row(journal: &Rows, key: &[u8]) -> TestResult<Value> {
    Ok(serde_json::from_slice(
        journal.get(key).ok_or("retained graph row")?,
    )?)
}

fn key(segment: &str, root: &str, ordinal: Option<usize>) -> TestResult<Vec<u8>> {
    let mut key = format!("j2\0{segment}\0{root}").into_bytes();
    if let Some(ordinal) = ordinal {
        key.push(0);
        key.extend_from_slice(&u32::try_from(ordinal)?.to_be_bytes());
    }
    Ok(key)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn include(journal: &Rows, expected: &mut Rows, key: Vec<u8>) -> TestResult {
    let raw = journal.get(&key).ok_or("expected graph physical row")?;
    check!(expected.insert(key, raw.clone()).is_none());
    Ok(())
}
