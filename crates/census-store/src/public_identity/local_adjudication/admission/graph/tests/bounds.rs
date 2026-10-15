use super::*;

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
    assert!(builder
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    assert_eq!(builder.descriptors.len(), MAX_DESCRIPTORS);
    assert!(builder.finish(&store, &generation.seal).is_err());
    assert_eq!(counters(&store)?, counters_before);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn descriptor_decoder_accepts_cumulative_bound_and_refuses_the_next_descriptor() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "descriptor-codec", plan)?;
    let descriptor = reservation.descriptors.first().ok_or("page descriptor")?;
    let bytes = codec::encode(descriptor, MAX_METADATA_BYTES)?.len();
    let maximum = MAX_DESCRIPTORS
        .min(MAX_PHYSICAL_ROWS / descriptor.physical.row_count())
        .min(MAX_METADATA_BYTES / bytes);
    assert!(maximum > 0);
    assert!(maximum < MAX_DESCRIPTORS);
    let descriptor = serde_json::to_value(descriptor)?;
    let mut raw = serde_json::to_value(&reservation)?;
    *raw.get_mut("descriptors").ok_or("descriptors")? =
        serde_json::Value::Array(vec![descriptor.clone(); maximum]);
    let decoded: GraphReservation = serde_json::from_slice(&serde_json::to_vec(&raw)?)?;
    assert_eq!(decoded.descriptors.len(), maximum);
    raw.get_mut("descriptors")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("descriptors")?
        .push(descriptor);
    let before = retained_state(&store)?;
    let error = serde_json::from_slice::<GraphReservation>(&serde_json::to_vec(&raw)?)
        .err()
        .ok_or("oversized descriptors accepted")?;
    assert!(error
        .to_string()
        .contains("cumulative graph admission bound"));
    assert_eq!(retained_state(&store)?, before);
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
    assert_eq!(exact.physical_rows, MAX_PHYSICAL_ROWS);
    let mut over = GraphPlanBuilder::new(generation.capture.clone());
    over.physical_rows = MAX_PHYSICAL_ROWS;
    assert!(over
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    assert!(over.descriptors.is_empty());
    assert_eq!(over.physical_rows, MAX_PHYSICAL_ROWS);
    let mut metadata = GraphPlanBuilder::new(generation.capture.clone());
    metadata.metadata_bytes = MAX_METADATA_BYTES;
    assert!(metadata
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    assert!(metadata.descriptors.is_empty());
    assert_eq!(metadata.metadata_bytes, MAX_METADATA_BYTES);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(counters(&store)?, (admission::HEADER_MARGIN, 0, 0));
    Ok(())
}

#[test]
fn missing_child_duplicate_child_or_unreachable_object_cannot_be_reserved() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, _plan) = Generation::new(&store, 0)?;
    let before = retained_state(&store)?;
    let mut builder = GraphPlanBuilder::new(generation.capture.clone());
    assert!(builder
        .add(&store, bridge::Node::Block(&generation.block))
        .is_err());
    assert!(builder
        .add(&store, bridge::Node::Seal(&generation.seal))
        .is_err());
    let page = builder.add(&store, bridge::Node::Page(&generation.page))?;
    assert!(builder
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    assert!(validate::children_present(&builder.descriptors, &[page.clone(), page]).is_err());
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
    assert!(builder.finish(&store, &generation.seal).is_err());
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(counters(&store)?, (admission::HEADER_MARGIN, 0, 0));
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
    assert!(reserve(&store, "oversized-graph", plan).is_err());
    assert!(load(&store, "oversized-graph")?.is_none());
    assert_eq!(counters(&store)?, (admission::HEADER_MARGIN, 0, 0));
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}
