use super::super::measure::{measure_with_limits, JsonByteLimits};
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

fn check_demand(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let before = retained_state(store)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    let report = snapshot.measure_frozen_public_identity_v2()?;
    let encoded = serde_json::to_value(&report)?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(eq; report.canonical_json_bytes(), frozen.bytes().len());
    check!(eq; encoded.get("operations").ok_or("missing operations")?, &serde_json::json!(bundle.operations.len()));
    check!(eq; encoded.get("objects").ok_or("missing objects")?, &serde_json::json!(bundle.objects.len()));
    let rows = bundle.objects.iter().try_fold(0usize, |count, object| {
        count
            .checked_add(object.rows.len())
            .ok_or("row count overflow")
    })?;
    check!(eq; encoded.get("physical_rows").ok_or("missing physical_rows")?, &serde_json::json!(rows));
    check!(eq; encoded.get("snapshot_sequence").ok_or("missing snapshot_sequence")?, &serde_json::json!(snapshot.sequence()));
    check!(eq; encoded.get("fits_normal_json_byte_limit").ok_or("missing fits_normal_json_byte_limit")?, &serde_json::json!(true));
    check!(eq; retained_state(store)?, before);
    Ok(())
}

#[test]
fn exact_demand_equals_real_frozen_empty_and_nonempty_bundle_lengths() -> TestResult {
    let (_directory, empty, _writer) = fixture()?;
    check_demand(&empty)?;
    let (_directory, single, writer) = fixture()?;
    let athlete = subject("demand-single", "705")?;
    single.append(Table::Athletes, &athlete)?;
    writer.establish_source_bound(
        &single,
        "demand-single",
        &athlete.id.cast(),
        single.snapshot().revision(),
        &writer.head(&single)?,
    )?;
    check_demand(&single)?;
    let (_directory, populated, _writer, _) = two_births()?;
    check_demand(&populated)
}

#[test]
fn diagnostic_boundary_refuses_partial_total_and_accepts_exact_total() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let held = store.db.snapshot();
    let bytes = snapshot.freeze_public_identity_v2()?.bytes().len();
    let before = retained_state(&store)?;
    for limit in [0, 1, bytes.checked_sub(1).ok_or("empty fixture")?] {
        check!(matches!(
            measure_with_limits(
                &held,
                &store.entities,
                &store.meta,
                &store.journal,
                &store.receipts,
                &accepted,
                JsonByteLimits { diagnostic: limit, normal: crate::MAX_FROZEN_PUBLIC_IDENTITY_BYTES },
            ),
            Err(crate::StoreError::BudgetExceeded { what: "frozen identity diagnostic canonical JSON bytes", needed, available }) if needed > available && available <= u64::try_from(limit)?
        ));
    }
    let exact = measure_with_limits(
        &held,
        &store.entities,
        &store.meta,
        &store.journal,
        &store.receipts,
        &accepted,
        JsonByteLimits {
            diagnostic: bytes,
            normal: crate::MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
        },
    )?;
    check!(eq; exact.canonical_json_bytes(), bytes);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn held_snapshot_demand_remains_bound_after_later_live_append() -> TestResult {
    let (_directory, store, writer, _) = two_births()?;
    let snapshot = store.snapshot();
    let before = snapshot
        .measure_frozen_public_identity_v2()?
        .canonical_json_bytes();
    let later = subject("demand-later", "704")?;
    store.append(Table::Athletes, &later)?;
    writer.establish_source_bound(
        &store,
        "demand-later",
        &later.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; snapshot.measure_frozen_public_identity_v2()?.canonical_json_bytes(), before);
    check!(
        store
            .snapshot()
            .measure_frozen_public_identity_v2()?
            .canonical_json_bytes()
            > before
    );
    Ok(())
}

#[test]
fn corrupt_original_graph_refuses_measurement_without_further_mutation() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let object = bundle.objects.first().ok_or("object")?;
    let key = crate::journal::object_manifest_key(object.reference.root().as_str())?;
    store.journal.insert(key, b"corrupt fixture manifest")?;
    let tampered = retained_state(&store)?;
    match store.snapshot().measure_frozen_public_identity_v2() {
        Err(crate::StoreError::Decode { key, .. })
            if key.starts_with("journal object manifest for ") =>
        {
            ()
        }
        Err(error) => return Err(format!("wrong corrupt manifest refusal: {error:?}").into()),
        Ok(_) => return Err("corrupt manifest measurement succeeded".into()),
    }
    check!(eq; retained_state(&store)?, tampered);
    Ok(())
}

fn sum_json<T: serde::Serialize>(values: &[T]) -> TestResult<usize> {
    values.iter().try_fold(0usize, |sum, value| {
        sum.checked_add(serde_json::to_vec(value)?.len())
            .ok_or("JSON sum overflow".into())
    })
}

#[test]
fn complete_diagnostic_continues_after_each_normal_component_exhaustion() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let held = store.db.snapshot();
    let frozen = snapshot.freeze_public_identity_v2()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let metadata = [
        bundle.declaration.len(),
        bundle.head.len(),
        bundle.history.len(),
    ]
    .into_iter()
    .try_fold(0usize, |sum, bytes| {
        sum.checked_add(bytes).ok_or("metadata sum overflow")
    })?;
    let operations = metadata
        .checked_add(sum_json(&bundle.operations)?)
        .ok_or("operation sum overflow")?;
    let object = bundle.objects.first().ok_or("first object")?;
    let mut first_physical = None;
    crate::public_identity::paged_generation::graph_physical_rows(
        &held,
        &store.journal,
        &object.reference,
        |key, value| {
            if first_physical.is_none() {
                let row = serde_json::json!({
                    "key_base64": STANDARD.encode(key),
                    "value_base64": STANDARD.encode(value),
                });
                first_physical = Some(
                    serde_json::to_vec(&row)
                        .map_err(|source| crate::StoreError::Json {
                            detail: "independent physical row length oracle".to_owned(),
                            source,
                        })?
                        .len(),
                );
            }
            Ok(())
        },
    )?;
    let first_row = first_physical.ok_or("first physical row")?;
    let row_limit = operations
        .checked_add(first_row / 2)
        .ok_or("row limit overflow")?;
    let object_limit = operations
        .checked_add(sum_json(&object.rows)?)
        .ok_or("object limit overflow")?;
    let before = retained_state(&store)?;
    let first_operation =
        serde_json::to_vec(bundle.operations.first().ok_or("first operation")?)?.len();
    let full_object = serde_json::to_vec(object)?.len();
    let physical_rows = bundle.objects.iter().try_fold(0usize, |sum, object| {
        sum.checked_add(object.rows.len())
            .ok_or("physical count overflow")
    })?;
    for (normal, component, complete_component_bytes, available_bytes) in [
        (0, "raw_metadata", bundle.declaration.len(), 0),
        (metadata, "operation_json", first_operation, 0),
        (row_limit, "physical_row_json", first_row, first_row / 2),
        (
            object_limit,
            "object_json",
            full_object,
            sum_json(&object.rows)?,
        ),
    ] {
        let report = measure_with_limits(
            &held,
            &store.entities,
            &store.meta,
            &store.journal,
            &store.receipts,
            &accepted,
            JsonByteLimits {
                diagnostic: frozen.bytes().len(),
                normal,
            },
        )?;
        check!(eq; report.canonical_json_bytes(), frozen.bytes().len());
        let value = serde_json::to_value(report)?;
        let accounting = value
            .get("normal_component_budget")
            .ok_or("normal budget")?;
        check!(eq; accounting, &serde_json::json!({ "status": "exceeded", "component": component, "complete_component_bytes": complete_component_bytes, "available_bytes": available_bytes }));
        check!(eq; value.get("operations"), Some(&serde_json::json!(bundle.operations.len())));
        check!(eq; value.get("objects"), Some(&serde_json::json!(bundle.objects.len())));
        check!(eq; value.get("normal_json_byte_limit"), Some(&serde_json::json!(normal)));
        check!(eq; value.get("physical_rows"), Some(&serde_json::json!(physical_rows)));
        let diagnostic = frozen.bytes().len().checked_sub(1).ok_or("empty fixture")?;
        check!(matches!(measure_with_limits(
            &held, &store.entities, &store.meta, &store.journal, &store.receipts, &accepted,
            JsonByteLimits { diagnostic, normal },
        ), Err(crate::StoreError::BudgetExceeded { what: "frozen identity diagnostic canonical JSON bytes", needed, available }) if needed > available && available <= u64::try_from(diagnostic)?));
    }
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
