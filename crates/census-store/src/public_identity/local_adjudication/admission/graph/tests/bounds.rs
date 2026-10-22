mod physical_rows;

use super::*;
use crate::public_identity::paged_generation::MAX_RAW_VALUE_BYTES;

#[test]
fn descriptor_bound_refuses_before_staging_or_changing_admission_counters() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let descriptor = plan.descriptors.first().ok_or("page descriptor")?.clone();
    let mut builder = GraphPlanBuilder::new(generation.capture.clone());
    builder.descriptors.resize(MAX_DESCRIPTORS, descriptor);
    let before = retained_state(&store)?;
    let counters_before = counters(&store)?;
    let sequence = store.snapshot().sequence();
    check!(builder
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    check!(eq;builder.descriptors.len(), MAX_DESCRIPTORS);
    check!(builder.finish(&store, &generation.seal).is_err());
    check!(eq;counters(&store)?, counters_before);
    check!(eq;retained_state(&store)?, before);
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn descriptor_decoder_accepts_cumulative_bound_and_refuses_the_next_descriptor() -> TestResult {
    for padding in [0, MAX_RAW_VALUE_BYTES] {
        descriptor_decoder_bound(padding)?;
    }
    Ok(())
}

fn descriptor_decoder_bound(padding: usize) -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_generation, plan) = Generation::new(&store, padding)?;
    let reservation = reserve(&store, "descriptor-codec", plan)?;
    let descriptor = reservation.descriptors.first().ok_or("page descriptor")?;
    let bytes = codec::encode(descriptor, MAX_METADATA_BYTES)?.len();
    let maximum = MAX_DESCRIPTORS
        .min(MAX_PHYSICAL_ROWS / descriptor.physical.row_count())
        .min(MAX_METADATA_BYTES / bytes);
    check!(maximum > 0);
    let expected = if maximum == MAX_DESCRIPTORS {
        "graph descriptor count"
    } else {
        "cumulative graph admission bound"
    };
    let descriptor = serde_json::to_value(descriptor)?;
    let mut raw = serde_json::to_value(&reservation)?;
    *raw.get_mut("descriptors").ok_or("descriptors")? =
        serde_json::Value::Array(vec![descriptor.clone(); maximum]);
    let decoded: GraphReservation = serde_json::from_slice(&serde_json::to_vec(&raw)?)?;
    check!(eq;decoded.descriptors.len(), maximum);
    raw.get_mut("descriptors")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("descriptors")?
        .push(descriptor);
    let before = retained_state(&store)?;
    let error = serde_json::from_slice::<GraphReservation>(&serde_json::to_vec(&raw)?)
        .err()
        .ok_or("oversized descriptors accepted")?;
    check!(error.to_string().contains(expected));
    check!(eq;retained_state(&store)?, before);
    Ok(())
}

#[test]
fn protected_reader_uses_graph_cap_and_preserves_nongraph_cap() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "protected-reader", plan)?;
    let key = admission::key("protected-reader");
    let length = crate::MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or("reader bound")?;
    let mut raw = codec::encode(&reservation, MAX_METADATA_BYTES)?;
    raw.resize(length, b' ');
    write(&store, key.as_bytes(), &raw)?;
    let identity = store.store_identity()?;
    let (bytes, _) = admission::audit_row(&store, &store.db.snapshot(), key.as_bytes(), &identity)?;
    check!(bytes >= u64::try_from(length)?);
    for version in [2, 4] {
        let mut raw = serde_json::to_vec(&serde_json::json!({"version": version}))?;
        raw.resize(length, b' ');
        write(&store, key.as_bytes(), &raw)?;
        match admission::audit_row(&store, &store.db.snapshot(), key.as_bytes(), &identity) {
            Err(StoreError::RowTooLarge { bytes, max }) => {
                check!(eq; bytes, length);
                check!(eq; max, crate::MAX_JOURNAL_VALUE_BYTES);
            }
            other => return Err(format!("expected nongraph byte refusal, got {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn physical_row_and_metadata_bounds_refuse_without_growing_builder_or_store() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, _plan) = Generation::new(&store, 0)?;
    let measured = bridge::Node::Page(&generation.page).measure(&store)?;
    let before = retained_state(&store)?;
    let mut exact = GraphPlanBuilder::new(generation.capture.clone());
    exact.physical_rows = MAX_PHYSICAL_ROWS
        .checked_sub(measured.row_count())
        .ok_or("row bound")?;
    exact.add(&store, bridge::Node::Page(&generation.page))?;
    check!(eq;exact.physical_rows, MAX_PHYSICAL_ROWS);
    let mut over = GraphPlanBuilder::new(generation.capture.clone());
    over.physical_rows = MAX_PHYSICAL_ROWS;
    check!(over
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    check!(over.descriptors.is_empty());
    check!(eq;over.physical_rows, MAX_PHYSICAL_ROWS);
    let mut metadata = GraphPlanBuilder::new(generation.capture.clone());
    metadata.metadata_bytes = MAX_METADATA_BYTES;
    check!(metadata
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    check!(metadata.descriptors.is_empty());
    check!(eq;metadata.metadata_bytes, MAX_METADATA_BYTES);
    check!(eq;retained_state(&store)?, before);
    check!(eq;counters(&store)?, (admission::HEADER_MARGIN, 0, 0));
    Ok(())
}

#[test]
fn missing_child_duplicate_child_or_unreachable_object_cannot_be_reserved() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, _plan) = Generation::new(&store, 0)?;
    let before = retained_state(&store)?;
    let mut builder = GraphPlanBuilder::new(generation.capture.clone());
    check!(builder
        .add(&store, bridge::Node::Block(&generation.block))
        .is_err());
    check!(builder
        .add(&store, bridge::Node::Seal(&generation.seal))
        .is_err());
    let page = builder.add(&store, bridge::Node::Page(&generation.page))?;
    check!(builder
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    check!(validate::children_present(&builder.descriptors, &[page.clone(), page]).is_err());
    let padding = generation
        .page
        .rows()
        .first()
        .ok_or("verdict row")?
        .raw_value()
        .len()
        .checked_add(1)
        .ok_or("padding")?;
    let (orphan, _plan) = Generation::new(&store, padding)?;
    builder.add(&store, bridge::Node::Page(&orphan.page))?;
    builder.add(&store, bridge::Node::Block(&generation.block))?;
    for table in &generation.tables {
        builder.add(&store, bridge::Node::Table(table))?;
    }
    check!(builder.finish(&store, &generation.seal).is_err());
    check!(eq;retained_state(&store)?, before);
    check!(eq;counters(&store)?, (admission::HEADER_MARGIN, 0, 0));
    Ok(())
}

#[test]
fn oversized_reservation_refuses_without_persisting_metadata_or_counters() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_generation, mut plan) = Generation::new(&store, 0)?;
    let descriptor = plan.descriptors.first().ok_or("page descriptor")?.clone();
    plan.descriptors.resize(
        MAX_DESCRIPTORS.checked_add(1).ok_or("descriptor bound")?,
        descriptor,
    );
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    check!(reserve(&store, "oversized-graph", plan).is_err());
    check!(load(&store, "oversized-graph")?.is_none());
    check!(eq;counters(&store)?, (admission::HEADER_MARGIN, 0, 0));
    check!(eq;retained_state(&store)?, before);
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}
