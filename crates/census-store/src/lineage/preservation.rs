use crate::{Store, Table};
use census_domain::model::StoreIdentity;

use super::process::bounded_case;
use super::support::{
    canonical, envelope, equal, fixture_dir, inject, lineage_rows, seed, TestResult, Witness, KEY,
    LOWER, MIXED, OLD_UPPER_ENVELOPE, PHASE, UPPER, ZERO,
};

#[test]
fn retained_historical_envelopes_are_adopted_without_any_write() -> TestResult {
    for value in [LOWER, UPPER, MIXED, ZERO] {
        let dir = fixture_dir()?;
        let store = Store::open_with(dir.path(), 8_388_608)?;
        seed(&store)?;
        let raw = envelope(value);
        inject(&store, &[canonical(&raw)])?;
        let before = Witness::capture(&store)?;
        equal(store.store_identity()?.as_str(), value)?;
        equal(store.snapshot().store_identity()?.as_str(), value)?;
        equal(String::from(store.store_identity()?), value.to_owned())?;
        equal(lineage_rows(&store)?, vec![canonical(&raw)])?;
        equal(&Witness::capture(&store)?, &before)?;
    }
    Ok(())
}

#[test]
fn genuine_old_uppercase_bytes_and_timestamp_survive_reopen_and_move() -> TestResult {
    let dir = fixture_dir()?;
    let original = dir.path().join("original");
    let moved = dir.path().join("moved");
    let before = {
        let store = Store::open_with(&original, 8_388_608)?;
        seed(&store)?;
        inject(&store, &[canonical(OLD_UPPER_ENVELOPE)])?;
        let before = Witness::capture(&store)?;
        equal(store.store_identity()?.as_str(), UPPER)?;
        equal(Witness::capture(&store)?, before)?;
        store.flush()?;
        Witness::capture(&store)?
    };
    {
        let store = Store::open_with(&original, 8_388_608)?;
        equal(store.store_identity()?.as_str(), UPPER)?;
        equal(&Witness::capture(&store)?, &before)?;
    }
    std::fs::rename(&original, &moved)?;
    let store = Store::open_with(&moved, 8_388_608)?;
    equal(store.root(), moved.as_path())?;
    equal(store.store_identity()?.as_str(), UPPER)?;
    equal(lineage_rows(&store)?, vec![canonical(OLD_UPPER_ENVELOPE)])?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn fresh_identity_creates_one_inline_record_and_advances_only_global_sequence() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    seed(&store)?;
    equal(lineage_rows(&store)?.len(), 0)?;
    let before = Witness::capture(&store)?;
    let identity = store.store_identity()?;
    if !identity
        .as_str()
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("a fresh identity is not lowercase SHA256: {identity:?}").into());
    }
    equal(identity.as_str().len(), 64)?;
    let after = Witness::capture(&store)?;
    before.unchanged_evidence(&after)?;
    equal(
        after.sequence,
        before.sequence.checked_add(1).ok_or("sequence overflow")?,
    )?;
    let records = lineage_rows(&store)?;
    equal(records.len(), 1)?;
    let (key, raw) = records
        .first()
        .ok_or("the fresh identity record is missing")?;
    equal(key.as_slice(), Store::journal_key(PHASE, KEY).as_slice())?;
    let entry: serde_json::Value = serde_json::from_slice(raw)?;
    let fields = entry
        .as_object()
        .ok_or("the fresh envelope is not an object")?;
    equal(fields.len(), 3)?;
    equal(fields.get("key"), Some(&serde_json::json!(KEY)))?;
    equal(
        fields.get("payload"),
        Some(&serde_json::json!(identity.as_str())),
    )?;
    let at = fields
        .get("at")
        .and_then(serde_json::Value::as_str)
        .ok_or("the fresh envelope has no string provenance timestamp")?;
    equal(
        chrono::DateTime::parse_from_rfc3339(at)?
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        at.to_owned(),
    )?;
    equal(store.snapshot().store_identity()?, identity.clone())?;
    equal(store.store_identity()?, identity)?;
    equal(Witness::capture(&store)?, after)?;
    equal(store.root().join(".export-lineage.lock").exists(), false)
}

#[test]
fn unknown_envelope_metadata_is_ignored_without_materializing_a_new_identity() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    seed(&store)?;
    let raw = format!(
        " {{\"metadata\":{{\"key\":\"foreign\",\"payload\":[null,{{\"opaque\":true}}]}},\"payload\":\"{MIXED}\",\"at\":\"1999-12-31\",\"key\":\"store-identity\"}} "
    ).into_bytes();
    inject(&store, &[canonical(&raw)])?;
    let before = Witness::capture(&store)?;
    equal(store.store_identity()?, StoreIdentity::parse(MIXED)?)?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn legacy_lock_files_are_untouched_and_do_not_gate_store_identity() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::legacy_lock_files_are_untouched_and_do_not_gate_store_identity"
        ),
        || {
            let dir = fixture_dir()?;
            let store = Store::open_with(dir.path(), 8_388_608)?;
            let path = store.root().join(".export-lineage.lock");
            std::fs::write(&path, b"retained legacy lock marker")?;
            let lock = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)?;
            lock.lock()?;
            let identity = store.store_identity()?;
            equal(store.snapshot().store_identity()?, identity)?;
            equal(
                std::fs::read(&path)?.as_slice(),
                b"retained legacy lock marker".as_slice(),
            )?;
            equal(
                store.snapshot().tables_digest(&[Table::Schools])?.as_str(),
                "a9b01f762859816b88a9cb6a0f1c7af64c1818ccc623ef02c6c4bb79950cb52c",
            )?;
            lock.unlock()?;
            Ok(())
        },
    )
}
