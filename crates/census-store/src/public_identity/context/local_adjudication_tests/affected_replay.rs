use super::*;
use crate::{AcceptedPublicIdentitySnapshot, Store, Table};
use census_domain::model::{CanonicalPerformance, RelayResult};
use census_domain::public_identity_history::HistoryRequest;
use serde_json::Value;

#[test]
fn affected_keys_and_full_raw_inventory_survive_reopen_and_frozen_replay_after_live_growth(
) -> TestResult {
    let fixture = merged()?;
    let expected_keys = results::expected_keys(&fixture)?;
    let canonical = fixture.births.first().ok_or("birth")?.public_id();
    let inventory = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-affected-replay",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    assert_eq!(
        prepared.record().affected_context,
        inventory.affected_context()
    );
    assert_eq!(
        prepared.record().request.affected_record_keys,
        expected_keys
    );
    let context_root = prepared.record().affected_context.clone();
    assert_full_raw_inventory(&fixture.store, prepared.record())?;
    let expected_context = context_bytes(&fixture.store, &context_root)?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest.clone())?;
    let frozen_before = fixture.store.snapshot().freeze_public_identity_v1()?;
    assert_frozen_context(&frozen_before, &context_root, &expected_context)?;
    assert_snapshot_keys(&frozen_before.replay()?, &expected_keys)?;
    let member = fixture.candidates.first().ok_or("member")?.cast();
    fixture.store.append(
        Table::Performances,
        &results::performance(&member, "later-live")?,
    )?;
    fixture
        .store
        .append(Table::RelayResults, &results::relay(&member, "later-live"))?;
    assert_eq!(
        results::raw::<CanonicalPerformance>(&fixture.store, Table::Performances)?.len(),
        6
    );
    assert_eq!(
        results::raw::<RelayResult>(&fixture.store, Table::RelayResults)?.len(),
        6
    );
    assert_eq!(
        context_bytes(&fixture.store, &context_root)?,
        expected_context
    );
    let grown_facts = results::retained_facts(&fixture.store)?;
    let root = fixture.directory.path().join("identity");
    let ReviewedFixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (reopened, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    assert_eq!(writer.head(&reopened)?, *outcome.head());
    assert_eq!(results::retained_facts(&reopened)?, grown_facts);
    assert_eq!(context_bytes(&reopened, &context_root)?, expected_context);
    let snapshot = reopened
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    assert_snapshot_keys(&snapshot, &expected_keys)?;
    let repeated = writer
        .replay_local_adjudication(&reopened, "synthetic-affected-replay", &digest)?
        .ok_or("replayed operation")?;
    assert!(repeated.replayed());
    assert_eq!(repeated.head(), outcome.head());
    let frozen_after = reopened.snapshot().freeze_public_identity_v1()?;
    assert_frozen_context(&frozen_after, &context_root, &expected_context)?;
    drop(writer);
    drop(reopened);
    drop(directory);
    assert_snapshot_keys(&frozen_before.replay()?, &expected_keys)?;
    assert_snapshot_keys(&frozen_after.replay()?, &expected_keys)?;
    Ok(())
}

fn assert_full_raw_inventory(
    store: &Store,
    record: &census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2,
) -> TestResult {
    let records = super::super::test_fixture::records(store, &record.affected_context)?;
    let header = records
        .first()
        .and_then(|row| row.pointer("/payload/value"))
        .ok_or("affected header")?;
    assert_eq!(header.get("counts"), Some(&serde_json::json!([5, 5])));
    assert_eq!(
        header.get("authority_context"),
        Some(&serde_json::json!(record.authority_context))
    );
    assert_eq!(
        header.get("snapshot_sequence"),
        Some(&serde_json::json!(record.snapshot_sequence))
    );
    assert_eq!(
        header.get("evidence_generation"),
        Some(&serde_json::json!(record.evidence_generation))
    );
    assert_eq!(
        header.get("derived_generation"),
        Some(&serde_json::json!(record.derived_generation))
    );
    let performances = retained_rows(&records, "performance")?;
    let relays = retained_rows(&records, "relay")?;
    assert_eq!(
        performances,
        serde_json::to_value(results::raw::<CanonicalPerformance>(
            store,
            Table::Performances
        )?)?
    );
    assert_eq!(
        relays,
        serde_json::to_value(results::raw::<RelayResult>(store, Table::RelayResults)?)?
    );
    Ok(())
}

fn retained_rows(records: &[Value], kind: &str) -> TestResult<Value> {
    let rows = records
        .iter()
        .filter(|row| row.pointer("/payload/kind").and_then(Value::as_str) == Some(kind))
        .map(|row| {
            let physical_key = row
                .pointer("/payload/value/physical_key_base64")
                .and_then(Value::as_str)
                .ok_or("physical key")?;
            assert!(!physical_key.is_empty());
            let raw = row
                .pointer("/payload/value/raw_json")
                .and_then(Value::as_str)
                .ok_or("raw result")?;
            Ok(serde_json::from_str(raw)?)
        })
        .collect::<TestResult<Vec<Value>>>()?;
    Ok(Value::Array(rows))
}

fn context_bytes(store: &Store, root: &str) -> TestResult<Vec<u8>> {
    Ok(
        crate::read::read_context_object(&store.db.snapshot(), &store.journal, root)?
            .retained_bytes(crate::journal::context::CONTEXT_TOTAL_BYTES)?,
    )
}

fn assert_frozen_context(
    frozen: &crate::FrozenPublicIdentityBundleV1,
    root: &str,
    expected: &[u8],
) -> TestResult {
    let wire: Value = serde_json::from_slice(frozen.bytes())?;
    let context = wire
        .get("contexts")
        .and_then(Value::as_array)
        .ok_or("contexts")?
        .iter()
        .find(|row| row.get("root").and_then(Value::as_str) == Some(root))
        .ok_or("affected context in frozen bundle")?;
    assert_eq!(
        context
            .get("bytes")
            .and_then(Value::as_str)
            .ok_or("frozen context bytes")?
            .as_bytes(),
        expected
    );
    Ok(())
}

fn assert_snapshot_keys(
    snapshot: &AcceptedPublicIdentitySnapshot,
    expected: &[String],
) -> TestResult {
    let last = snapshot.history_records().last().ok_or("split history")?;
    let HistoryRequest::Split(split) = &last.request else {
        return Err("last history operation is not a split".into());
    };
    assert_eq!(split.affected_record_keys, expected);
    Ok(())
}
