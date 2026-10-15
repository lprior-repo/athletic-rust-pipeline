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
    let inventory = fixture.writer.inspect_local_adjudication(
        &fixture.store,
        "synthetic-affected-replay",
        canonical,
    )?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-affected-replay",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    assert_eq!(prepared.graph(), inventory.graph());
    assert_eq!(
        prepared.record().request.affected_record_keys,
        expected_keys
    );
    let graph = prepared.graph().clone();
    assert_full_raw_inventory(&fixture.store, &graph)?;
    let expected_objects = graph_objects(&fixture.store, &graph)?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest.clone())?;
    let frozen_before = fixture.store.snapshot().freeze_public_identity_v2()?;
    assert_frozen_objects(&frozen_before, &expected_objects)?;
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
    assert_eq!(graph_objects(&fixture.store, &graph)?, expected_objects);
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
    let (reopened, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    assert_eq!(writer.head(&reopened)?, *outcome.head());
    assert_eq!(results::retained_facts(&reopened)?, grown_facts);
    assert_eq!(graph_objects(&reopened, &graph)?, expected_objects);
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
    let frozen_after = reopened.snapshot().freeze_public_identity_v2()?;
    assert_frozen_objects(&frozen_after, &expected_objects)?;
    drop(writer);
    drop(reopened);
    drop(directory);
    assert_snapshot_keys(&frozen_before.replay()?, &expected_keys)?;
    assert_snapshot_keys(&frozen_after.replay()?, &expected_keys)?;
    Ok(())
}

fn assert_full_raw_inventory(
    store: &Store,
    reference: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
) -> TestResult {
    use crate::public_identity::paged_generation::{AuthorityTableV2, VerifiedRawGraph};
    let snapshot = store.snapshot();
    let graph = VerifiedRawGraph::verify(&snapshot, reference.capture(), reference.seal())?;
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    graph.for_each_physical::<CanonicalPerformance>(
        AuthorityTableV2::Performances,
        super::super::BUDGET,
        |row, key, raw, ordinal, _| {
            performances.push((row, key.to_vec(), raw.to_vec(), ordinal));
            Ok(())
        },
    )?;
    graph.for_each_physical::<RelayResult>(
        AuthorityTableV2::RelayResults,
        super::super::BUDGET,
        |row, key, raw, ordinal, _| {
            relays.push((row, key.to_vec(), raw.to_vec(), ordinal));
            Ok(())
        },
    )?;
    let performances = checked_rows(performances)?;
    let relays = checked_rows(relays)?;
    check!(eq; performances.len(), 5);
    check!(eq; relays.len(), 5);
    check!(eq; serde_json::to_value(performances)?, serde_json::to_value(results::raw::<CanonicalPerformance>(store, Table::Performances)?)?);
    check!(eq; serde_json::to_value(relays)?, serde_json::to_value(results::raw::<RelayResult>(store, Table::RelayResults)?)?);
    Ok(())
}

fn checked_rows<T: serde::Serialize>(rows: Vec<(T, Vec<u8>, Vec<u8>, u64)>) -> TestResult<Vec<T>> {
    rows.into_iter()
        .enumerate()
        .map(|(index, (row, key, raw, ordinal))| {
            check!(eq; ordinal, u64::try_from(index)?);
            check!(!key.is_empty());
            check!(eq; serde_json::to_value(&row)?, serde_json::from_slice::<Value>(&raw)?);
            Ok(row)
        })
        .collect()
}

fn graph_objects(
    store: &Store,
    reference: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
) -> TestResult<Vec<Value>> {
    use crate::public_identity::paged_generation::{capture::verify, graph_physical_rows};
    use base64::Engine;
    let mut objects = std::collections::BTreeMap::new();
    verify::verify(
        &store.snapshot(),
        reference.capture(),
        reference.seal(),
        |_, object| {
            let mut rows = std::collections::BTreeMap::new();
            graph_physical_rows(
                &store.db.snapshot(),
                &store.journal,
                object,
                |key, value| {
                    let row = serde_json::json!({
                        "key_base64": base64::engine::general_purpose::STANDARD.encode(key),
                        "value_base64": base64::engine::general_purpose::STANDARD.encode(value),
                    });
                    if rows.insert(key.to_vec(), row).is_some() {
                        return Err(crate::PublicIdentityError::Corrupt(
                            "duplicate expected graph physical row",
                        )
                        .into());
                    }
                    Ok(())
                },
            )?;
            let rows: Vec<_> = rows.into_values().collect();
            let expected = serde_json::json!({"reference": object, "rows": rows});
            if let Some(previous) =
                objects.insert(object.root().as_str().to_owned(), expected.clone())
            {
                if previous != expected {
                    return Err(crate::PublicIdentityError::Corrupt(
                        "conflicting expected graph object",
                    )
                    .into());
                }
            }
            Ok(())
        },
    )?;
    Ok(objects.into_values().collect())
}

fn assert_frozen_objects(
    frozen: &crate::FrozenPublicIdentityBundleV2,
    expected: &[Value],
) -> TestResult {
    let wire: Value = serde_json::from_slice(frozen.bytes())?;
    let objects = wire
        .get("objects")
        .and_then(Value::as_array)
        .ok_or("frozen graph objects")?;
    for object in expected {
        check!(objects.contains(object));
    }
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
