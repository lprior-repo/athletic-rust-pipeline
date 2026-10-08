use super::*;

const PHASE: &str = "milesplit_owned_meet_v1";
const BASE: &str = "row/699482/immutable-capture/data[0]";

fn owned_row(time: Value) -> Value {
    json!({"locator":"data[0]","result_id":1,"meet_id":699482,"result_set_id":2,
        "source_athlete":{"namespace":{"Milesplit":{"kind":"athlete"}},"id":"3"},
        "team_id":4,"first_name":"Runner","last_name":"Historic","gender":"Female",
        "event_kind":"Track100m","mark":{"TimeSeconds":time},"timing":"fat",
        "provider":{"mark":"10.941","TimeSeconds":1094,"eventName":"100 Meter Dash","id":1}})
}

fn digest(payload: &Value) -> TestResult<String> {
    Ok(census_domain::model::serialized_digest(payload)?)
}

#[test]
fn canonical_owned_journal_rows_rekey_by_payload_hash_and_archive_original_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = owned_row(json!(1094));
    let old_key = format!("{BASE}/{}", digest(&original)?);
    store.journal_done(PHASE, &old_key, &original)?;
    let original_bytes = store
        .journal
        .get(Store::journal_key(PHASE, &old_key))?
        .ok_or("original owned row")?
        .to_vec();
    version(&store, "1")?;
    drop(store);
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.rewritten_rows, report.dropped_rows), (1, 0));
    let store = Store::open(dir.path())?;
    check!(eq; store.journal_payload(PHASE, &old_key)?, None);
    let mut expected = original;
    expected["mark"]["TimeSeconds"] = expected_time();
    let new_key = format!("{BASE}/{}", digest(&expected)?);
    check!(eq; store.journal_payload(PHASE, &new_key)?, Some(expected));
    let archive = Store::journal_key(
        super::super::owned::ARCHIVE_PHASE,
        &format!("original/{PHASE}/{old_key}"),
    );
    check!(eq; store.journal.get(archive)?.ok_or("schema1 lineage")?.to_vec(), original_bytes);
    check!(eq; meta::get_u64(&store.meta, "schema:exacttime:archived_owned_rows")?, Some(1));
    check!(eq; store.journal_payloads(PHASE)?.len(), 1);
    drop(store);
    check!(eq; Store::migrate(dir.path())?.rewritten_rows, 0);
    Ok(())
}

#[test]
fn digest_mismatch_and_malformed_owned_time_do_not_remove_or_archive_any_source_row() -> TestResult
{
    [owned_row(json!(0)), owned_row(json!(1094))]
        .into_iter()
        .enumerate()
        .try_for_each(|(index, payload)| -> TestResult {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let suffix = if index == 0 {
                digest(&payload)?
            } else {
                "a".repeat(64)
            };
            store.journal_done(PHASE, &format!("{BASE}/{suffix}"), &payload)?;
            version(&store, "1")?;
            let original = all_bytes(&store.journal)?;
            drop(store);
            check!(matches!(
                Store::migrate(dir.path()),
                Err(StoreError::Refused { .. })
            ));
            let (_, _, journal, _, _) =
                crate::open_keyspaces(dir.path(), crate::format::DEFAULT_CACHE_BYTES)?;
            check!(eq; all_bytes(&journal)?, original);
            drop(journal);
            check!(eq; Store::format(dir.path())?.migration_target, Some(2));
            Ok(())
        })
}

#[test]
fn legacy_unhashed_owned_rows_migrate_and_unrelated_raw_mark_keys_remain_byte_identical(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = owned_row(json!(1094));
    store.journal_done(PHASE, BASE, &original)?;
    store.journal_done(
        "raw_provider",
        "row/raw",
        &json!({"mark":{"TimeSeconds":1094}}),
    )?;
    store.journal_done(
        PHASE,
        "rejected/capture/data[1]",
        &json!({"provider":{"mark":{"TimeSeconds":1094}}}),
    )?;
    let raw = store
        .journal
        .get(Store::journal_key("raw_provider", "row/raw"))?
        .ok_or("raw provider")?
        .to_vec();
    let rejected = store
        .journal
        .get(Store::journal_key(PHASE, "rejected/capture/data[1]"))?
        .ok_or("raw rejection")?
        .to_vec();
    version(&store, "1")?;
    drop(store);
    Store::migrate(dir.path())?;
    let store = Store::open(dir.path())?;
    check!(eq; store.journal.get(Store::journal_key("raw_provider", "row/raw"))?.ok_or("raw retained")?.to_vec(), raw);
    check!(eq; store.journal.get(Store::journal_key(PHASE, "rejected/capture/data[1]"))?.ok_or("rejection retained")?.to_vec(), rejected);
    let mut expected = original;
    expected["mark"]["TimeSeconds"] = expected_time();
    check!(eq; store.journal_payload(PHASE, &format!("{BASE}/{}", digest(&expected)?))?, Some(expected));
    Ok(())
}
