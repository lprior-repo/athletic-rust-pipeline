use super::*;

#[test]
fn graph_permits_refuse_unlisted_root_wrong_root_wrong_store_and_unready_parent() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_other_directory, other, _other_writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "permit-scope", plan)?;
    let page_root = reservation
        .descriptors
        .first()
        .ok_or("page")?
        .object
        .root()
        .as_str();
    let block_root = reservation
        .descriptors
        .get(1)
        .ok_or("block")?
        .object
        .root()
        .as_str();
    let before = retained_state(&store)?;
    let other_before = retained_state(&other)?;
    assert!(reservation.permit_for(&store, &"f".repeat(64)).is_err());
    assert!(reservation.permit_for(&other, page_root).is_err());
    assert!(reservation.permit_for(&store, block_root).is_err());
    assert!(reservation
        .permit_for(&store, reservation.seal().root().as_str())
        .is_err());
    let page_permit = reservation.permit_for(&store, page_root)?;
    assert!(
        matches!(bridge::Node::Block(&generation.block).stage(&store, &page_permit), Err(StoreError::Refused { detail }) if detail.contains("reserved context root changed"))
    );
    assert!(bridge::Node::Page(&generation.page)
        .stage(&other, &page_permit)
        .is_err());
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(retained_state(&other)?, other_before);
    bridge::Node::Page(&generation.page).stage(&store, &page_permit)?;
    let block_permit = reservation.permit_for(&store, block_root)?;
    bridge::Node::Block(&generation.block).stage(&store, &block_permit)?;
    assert!(!reservation.is_complete(&store)?);
    Ok(())
}

#[test]
fn missing_or_corrupted_ready_descendant_refuses_audit_without_repair_or_recharge() -> TestResult {
    for corrupt in [false, true] {
        let (_directory, store, _writer) = enrolled()?;
        let (generation, plan) = Generation::new(&store, 0)?;
        let reservation = reserve(&store, "damaged-ready", plan)?;
        generation.stage(&store, &reservation)?;
        assert!(reservation.is_complete(&store)?);
        let root = reservation
            .descriptors
            .first()
            .ok_or("page")?
            .object
            .root()
            .as_str();
        let key = crate::journal::object_chunk_key(root, 0)?;
        let header = header_bytes(&store)?;
        if corrupt {
            let mut raw = row(&store, &key)?.ok_or("ready chunk")?;
            let byte = raw.last_mut().ok_or("chunk tail")?;
            *byte ^= 1;
            write(&store, &key, &raw)?;
        } else {
            remove(&store, &[key])?;
        }
        let damaged = retained_state(&store)?;
        let sequence = store.snapshot().sequence();
        assert!(load(&store, "damaged-ready").is_err());
        assert!(reservation.permit_for(&store, root).is_err());
        assert!(generation.plan(&store).is_err());
        assert!(reservation.is_complete(&store).is_err());
        assert_eq!(header_bytes(&store)?, header);
        assert_eq!(retained_state(&store)?, damaged);
        assert_eq!(store.snapshot().sequence(), sequence);
    }
    Ok(())
}

#[test]
fn extra_partial_descendant_row_refuses_exact_inventory_audit_without_writes() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "extra-partial", plan)?;
    let root = reservation
        .descriptors
        .first()
        .ok_or("page")?
        .object
        .root()
        .as_str();
    let header = header_bytes(&store)?;
    write(
        &store,
        &crate::journal::object_chunk_key(root, 1)?,
        b"extra",
    )?;
    assert!(row(&store, &crate::journal::object_manifest_key(root)?)?.is_none());
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(load(&store, "extra-partial").is_err());
    assert!(reservation.permit_for(&store, root).is_err());
    assert!(generation.plan(&store).is_err());
    assert_eq!(header_bytes(&store)?, header);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn changed_capture_duplicate_operation_and_unenrolled_store_refuse_without_writes() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "fixed-operation", plan)?;
    let before = retained_state(&store)?;
    assert!(reserve(&store, "fixed-operation", generation.plan(&store)?).is_err());
    let mut changed = generation.plan(&store)?;
    changed.capture = CaptureBindingV2::new(
        store.store_identity()?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(10, 7, 12, [3, 4, 5, 6, 7, 8]),
    )?;
    assert!(reservation.require_plan(&changed).is_err());
    assert_eq!(retained_state(&store)?, before);
    let (_directory, unopened, _writer) = fixture()?;
    let (_, plan) = Generation::new(&unopened, 0)?;
    let before = retained_state(&unopened)?;
    assert!(reserve(&unopened, "not-enrolled", plan).is_err());
    assert_eq!(retained_state(&unopened)?, before);
    Ok(())
}

#[test]
fn ready_seal_cannot_hide_a_missing_descendant_manifest() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "sealed-missing-descendant", plan)?;
    generation.stage(&store, &reservation)?;
    let root = reservation
        .descriptors
        .first()
        .ok_or("page")?
        .object
        .root()
        .as_str();
    let manifest = crate::journal::object_manifest_key(root)?;
    remove(&store, &[manifest])?;
    assert!(row(
        &store,
        &crate::journal::object_manifest_key(reservation.seal().root().as_str())?
    )?
    .is_some());
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(reservation.is_complete(&store).is_err());
    assert!(load(&store, "sealed-missing-descendant").is_err());
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn zero_charge_ready_permit_cannot_recreate_a_manifest_removed_after_issuance() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let original = reserve(&store, "ready-original", plan)?;
    generation.stage(&store, &original)?;
    let reused = reserve(&store, "ready-reused", generation.plan(&store)?)?;
    let leaf = reused.descriptors.first().ok_or("page")?;
    assert_eq!((leaf.incremental.bytes, leaf.incremental.roots), (0, 0));
    let root = leaf.object.root().as_str();
    let held_permit = reused.permit_for(&store, root)?;
    let header = header_bytes(&store)?;
    let manifest = crate::journal::object_manifest_key(root)?;
    remove(&store, &[manifest.clone()])?;
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(reused.permit_for(&store, root).is_err());
    assert!(bridge::Node::Page(&generation.page)
        .stage(&store, &held_permit)
        .is_err());
    assert!(row(&store, &manifest)?.is_none());
    assert_eq!(header_bytes(&store)?, header);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn seal_permit_refuses_missing_transitive_descendant_even_when_tables_are_ready() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "seal-transitive-child", plan)?;
    stage(&store, &reservation, bridge::Node::Page(&generation.page))?;
    stage(&store, &reservation, bridge::Node::Block(&generation.block))?;
    for table in &generation.tables {
        stage(&store, &reservation, bridge::Node::Table(table))?;
    }
    for table in generation.seal.tables() {
        assert!(row(
            &store,
            &crate::journal::object_manifest_key(table.object().root().as_str())?
        )?
        .is_some());
    }
    let root = reservation
        .descriptors
        .first()
        .ok_or("page")?
        .object
        .root()
        .as_str();
    remove(&store, &[crate::journal::object_manifest_key(root)?])?;
    let before = retained_state(&store)?;
    let header = header_bytes(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(reservation
        .permit_for(&store, reservation.seal().root().as_str())
        .is_err());
    assert!(row(
        &store,
        &crate::journal::object_manifest_key(reservation.seal().root().as_str())?
    )?
    .is_none());
    assert_eq!(header_bytes(&store)?, header);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn ready_page_from_another_table_revision_cannot_enter_a_new_capture_plan() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "original-capture", plan)?;
    generation.stage(&store, &reservation)?;
    let changed = CaptureBindingV2::new(
        store.store_identity()?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(10, 8, 12, [3, 4, 6, 7, 7, 8]),
    )?;
    let mut builder = GraphPlanBuilder::new(changed);
    let before = retained_state(&store)?;
    let charged = counters(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(builder
        .add(&store, bridge::Node::Page(&generation.page))
        .is_err());
    assert!(builder.descriptors.is_empty());
    assert_eq!(counters(&store)?, charged);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn same_capture_block_cannot_relabel_a_verdict_page_as_an_athlete_page() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, _plan) = Generation::new(&store, 0)?;
    let mut builder = GraphPlanBuilder::new(generation.capture.clone());
    let page = builder.add(&store, bridge::Node::Page(&generation.page))?;
    let reference = generation.page.reference(page)?;
    let athletes = generation
        .capture
        .table_binding(AuthorityTableV2::Athletes)?;
    let relabeled = RawPageBlockV2::new(&athletes, 0, vec![reference])?;
    let before = retained_state(&store)?;
    let charged = counters(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(builder
        .add(&store, bridge::Node::Block(&relabeled))
        .is_err());
    assert_eq!(builder.descriptors.len(), 1);
    assert_eq!(counters(&store)?, charged);
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}
