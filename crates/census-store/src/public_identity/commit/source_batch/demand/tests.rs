use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::Table;

fn stage(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
) -> TestResult<GraphAuthorityReferenceV1> {
    let request = Request::new(writer.store_identity(), operation, GradYear::CO2027)?;
    let capture = graph_operation::begin(
        writer,
        store,
        &request.operation,
        &request.digest,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    Ok(capture.graph)
}

fn measure(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
) -> StoreResult<SourceCohortDemandV1> {
    writer.measure_retained_source_cohort(
        store,
        operation,
        GradYear::CO2027,
        store.snapshot().revision(),
        &writer.head(store)?,
    )
}

#[test]
fn missing_original_graph_refuses_without_staging_even_when_outer_key_exists() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    store.append(Table::Athletes, &subject("demand-missing", "99001")?)?;
    let raw = graph_operation::raw_operation(writer.store_identity(), "demand-missing")?;
    writer.capture_raw_generation_v2(&store, &raw)?;
    let before = retained_state(&store)?;
    check!(measure(&store, &writer, "demand-missing").is_err());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn complete_original_graph_measurement_matches_literal_entry_and_changes_no_state() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("demand-complete", "99002")?;
    store.append_many(Table::Athletes, &[athlete.clone(), athlete.clone()])?;
    let graph = stage(&store, &writer, "demand-complete")?;
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    let report = serde_json::to_value(measure(&store, &writer, "demand-complete")?)?;
    check!(eq; report.get("graph"), Some(&serde_json::to_value(&graph)?));
    check!(eq; report.pointer("/authority/status"), Some(&serde_json::json!("complete")));
    let literal = serde_json::json!({"key":"athlete:00000000000000000000","at":"",
        "payload":{"kind":"athlete","value":athlete}});
    let bytes = serde_json::to_vec(&literal)?
        .len()
        .checked_add(1)
        .ok_or("entry bytes")?;
    check!(eq; report.pointer("/authority/sections/0/rows"), Some(&serde_json::json!(1)));
    check!(eq; report.pointer("/authority/sections/0/bytes"), Some(&serde_json::json!(bytes)));
    check!(eq; report.pointer("/authority/sections/0/scan"), Some(&serde_json::json!("complete")));
    check!(eq; report.pointer("/authority/tables/0/body_verified_inventory/rows"), Some(&serde_json::json!(2)));
    check!(eq; report.get("requested_cohort"), Some(&serde_json::json!(2027)));
    check!(eq; report.pointer("/authority/ordinary_raw_bytes_per_table"), Some(&serde_json::json!(8_589_934_592_u64)));
    check!(eq; writer.head(&store)?.sequence, 0);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn moved_inputs_and_stale_revision_refuse_without_new_capture_or_authority() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    store.append(Table::Athletes, &subject("demand-old", "99003")?)?;
    stage(&store, &writer, "demand-moved")?;
    let stale = store.snapshot();
    store.append(Table::Athletes, &subject("demand-new", "99004")?)?;
    let before = retained_state(&store)?;
    check!(measure(&store, &writer, "demand-moved").is_err());
    check!(writer
        .measure_retained_source_cohort(
            &store,
            "demand-moved",
            GradYear::CO2027,
            stale.revision(),
            &writer.head(&store)?
        )
        .is_err());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn missing_seal_readiness_and_foreign_revision_refuse_without_mutation() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let (_foreign_directory, foreign, _foreign_writer) = fixture()?;
    store.append(Table::Athletes, &subject("demand-incomplete", "99005")?)?;
    let graph = stage(&store, &writer, "demand-incomplete")?;
    let before = retained_state(&store)?;
    check!(writer
        .measure_retained_source_cohort(
            &store,
            "demand-incomplete",
            GradYear::CO2027,
            foreign.snapshot().revision(),
            &writer.head(&store)?
        )
        .is_err());
    check!(eq; retained_state(&store)?, before);
    let key = crate::journal::object_manifest_key(graph.seal().root().as_str())?;
    let mut batch = store.db.batch();
    batch.remove(&store.journal, key);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let before = retained_state(&store)?;
    check!(measure(&store, &writer, "demand-incomplete").is_err());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn requested_year_is_labeled_and_does_not_reinterpret_normal_cohort_replay() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    store.append(Table::Athletes, &subject("demand-year", "99006")?)?;
    stage(&store, &writer, "demand-year")?;
    let before = retained_state(&store)?;
    let other = GradYear::new(2028).ok_or("year")?;
    let report = writer.measure_retained_source_cohort(
        &store,
        "demand-year",
        other,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let report = serde_json::to_value(report)?;
    check!(eq; report.get("requested_cohort"), Some(&serde_json::json!(2028)));
    check!(report
        .get("scope")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| text.contains("not authenticated by the raw graph")));
    check!(eq; retained_state(&store)?, before);
    let outcomes = writer.establish_source_bound_cohort(
        &store,
        "demand-year",
        GradYear::CO2027,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; outcomes.len(), 1);
    let before = retained_state(&store)?;
    check!(writer
        .establish_source_bound_cohort(
            &store,
            "demand-year",
            other,
            store.snapshot().revision(),
            &writer.head(&store)?
        )
        .is_err());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn retained_measurement_cold_reopens_and_refuses_changed_head_without_writes() -> TestResult {
    let (directory, store, writer) = fixture()?;
    store.append(Table::Athletes, &subject("demand-cold", "99007")?)?;
    stage(&store, &writer, "demand-cold")?;
    let initial = serde_json::to_value(measure(&store, &writer, "demand-cold")?)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let before = retained_state(&store)?;
    check!(eq; serde_json::to_value(measure(&store, &writer, "demand-cold")?)?, initial);
    let mut wrong_head = writer.head(&store)?;
    wrong_head.sequence = wrong_head.sequence.checked_add(1).ok_or("head")?;
    check!(matches!(
        writer.measure_retained_source_cohort(
            &store,
            "demand-cold",
            GradYear::CO2027,
            store.snapshot().revision(),
            &wrong_head
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::HeadMoved))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn oversized_merged_entry_reports_exact_provisional_prefix_and_preserves_store() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let left = subject("demand-prefix", "99008")?;
    let right = subject("demand-overlarge", "99009")?;
    let (first, mut large) = if left.id.as_str() < right.id.as_str() {
        (left, right)
    } else {
        (right, left)
    };
    let mut other = large.clone();
    large.evidence.push(census_domain::model::Evidence::parsed(
        census_domain::model::SourceRef::id("a".repeat(600_000)),
        "2026-10-23",
    ));
    other.evidence.push(census_domain::model::Evidence::parsed(
        census_domain::model::SourceRef::id("b".repeat(600_000)),
        "2026-10-23",
    ));
    store.append_many(Table::Athletes, &[first.clone(), large, other])?;
    stage(&store, &writer, "demand-overlarge")?;
    let before = retained_state(&store)?;
    let report = serde_json::to_value(measure(&store, &writer, "demand-overlarge")?)?;
    check!(eq; report.pointer("/authority/status/stopped/table"), Some(&serde_json::json!("athletes")));
    check!(eq; report.pointer("/authority/sections/0/scan"), Some(&serde_json::json!("provisional")));
    check!(eq; report.pointer("/authority/sections/0/rows"), Some(&serde_json::json!(1)));
    let literal = serde_json::json!({"key":"athlete:00000000000000000000","at":"",
        "payload":{"kind":"athlete","value":first}});
    let bytes = serde_json::to_vec(&literal)?
        .len()
        .checked_add(1)
        .ok_or("prefix bytes")?;
    check!(eq; report.pointer("/authority/sections/0/bytes"), Some(&serde_json::json!(bytes)));
    check!(eq; report.pointer("/authority/sections/0/max_entry_bytes"), Some(&serde_json::json!(bytes)));
    check!(eq; report.pointer("/authority/authority_entry_bytes"), Some(&serde_json::json!(bytes)));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn retained_preparation_with_other_request_digest_refuses_without_writes() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("demand-prepared", "99011")?;
    store.append(Table::Athletes, &athlete)?;
    let request = Request::new(writer.store_identity(), "demand-prepared", GradYear::CO2027)?;
    crate::public_identity::graph_operation::tests::stage_source_preparation(
        &store,
        &writer,
        &request.operation,
        &athlete.id.cast(),
    )?;
    let before = retained_state(&store)?;
    check!(matches!(
        measure(&store, &writer, "demand-prepared"),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
