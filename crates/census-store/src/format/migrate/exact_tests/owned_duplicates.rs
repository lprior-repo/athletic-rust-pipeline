use super::*;

#[test]
fn duplicate_owned_payloads_keep_existing_current_envelope_and_archive_each_original_timestamp(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let base = "row/699482/capture/data[0]";
    let phase = "milesplit_owned_meet_v1";
    let original = json!({"result_id":1,"mark":{"TimeSeconds":1094},"provider":{"mark":"10.941"}});
    let old_key = format!(
        "{base}/{}",
        census_domain::model::serialized_digest(&original)?
    );
    let current =
        json!({"result_id":1,"mark":{"TimeSeconds":expected_time()},"provider":{"mark":"10.941"}});
    let new_key = format!(
        "{base}/{}",
        census_domain::model::serialized_digest(&current)?
    );
    let first = serde_json::to_vec(&json!({"key":base,"at":"2026-01-01","payload":original}))?;
    let second = serde_json::to_vec(&json!({"key":old_key,"at":"2026-02-01","payload":original}))?;
    let standing = serde_json::to_vec(&json!({"key":new_key,"at":"2026-03-01","payload":current}))?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, Store::journal_key(phase, base), &first);
    batch.insert(&store.journal, Store::journal_key(phase, &old_key), &second);
    batch.insert(
        &store.journal,
        Store::journal_key(phase, &new_key),
        &standing,
    );
    meta::put_text(&mut batch, &store.meta, "schema:store_version", "1");
    batch.commit()?;
    drop(store);
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.rewritten_rows, report.dropped_rows), (2, 0));
    let store = Store::open(dir.path())?;
    check!(eq; store.journal.get(Store::journal_key(phase, &new_key))?.ok_or("current envelope")?.to_vec(), standing);
    let archive_phase = super::super::owned::ARCHIVE_PHASE;
    check!(eq; store.journal.get(Store::journal_key(archive_phase, &format!("original/{phase}/{base}")))?.ok_or("first timestamp")?.to_vec(), first);
    check!(eq; store.journal.get(Store::journal_key(archive_phase, &format!("original/{phase}/{old_key}")))?.ok_or("second timestamp")?.to_vec(), second);
    check!(eq; store.journal_payload(phase, base)?, None);
    check!(eq; store.journal_payload(phase, &old_key)?, None);
    check!(eq; meta::get_u64(&store.meta, "schema:exacttime:archived_owned_rows")?, Some(2));
    Ok(())
}
