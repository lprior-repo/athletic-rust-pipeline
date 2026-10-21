use super::*;

fn reserve_partial(store: &Store, id: &RawGenerationOperationIdV2) -> TestResult<GraphReservation> {
    let guard = store.lock_appends();
    let held = store.snapshot_locked();
    let binding = binding::capture(store, &held)?;
    let mut builder = GraphPlanBuilder::new(binding.clone());
    let seal = scan::walk(&held, &binding, |node| builder.add(store, node))?;
    let reserved = reserve_graph(store, id, builder.finish(store, &seal)?)?;
    drop(guard);
    let mut first = true;
    let result = scan::walk(&held, &binding, |node| {
        if !first {
            return Err(super::super::super::invalid("test interruption"));
        }
        first = false;
        stage_node(store, &reserved, node)
    });
    check!(result.is_err());
    check!(!reserved.is_complete(store)?);
    Ok(reserved)
}

#[test]
fn partial_capture_reopens_and_resumes_original_reserved_content() -> TestResult {
    let (directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let id = operation("resume")?;
    let reserved = reserve_partial(&store, &id)?;
    let expected = reserved.seal().clone();
    drop(reserved);
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let outcome = writer.capture_raw_generation_v2(&store, &id)?;
    check!(matches!(
        &outcome,
        RawGenerationCaptureOutcomeV2::Captured(_)
    ));
    let retained = outcome.retained().clone();
    check!(eq;retained.seal(), &expected);
    check!(eq;retained_rows(&store, &retained)?, 513);
    let sequence = store.snapshot().sequence();
    let replay = writer.capture_raw_generation_v2(&store, &id)?;
    check!(matches!(
        &replay,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    check!(eq;replay.retained(), &retained);
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn complete_capture_reopens_without_original_source_rows() -> TestResult {
    let (directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let id = operation("source-absent")?;
    let retained = writer
        .capture_raw_generation_v2(&store, &id)?
        .retained()
        .clone();
    let mut batch = store.db.batch();
    for index in 0..513 {
        batch.remove(&store.entities, raw(index)?.0);
    }
    marks(&store, &mut batch, 0);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq;retained_rows(&store, &retained)?, 513);
    let replay = writer.capture_raw_generation_v2(&store, &id)?;
    check!(matches!(
        &replay,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    check!(eq;replay.retained(), &retained);
    Ok(())
}

#[test]
fn incomplete_retry_refuses_moved_inputs_without_writing_or_reclaiming() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let id = operation("moved-input")?;
    let reserved = reserve_partial(&store, &id)?;
    let mut batch = store.db.batch();
    crate::input_revisions::stage(&store, &mut batch, [Table::Athletes])?;
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let sequence = store.snapshot().sequence();
    check!(writer.capture_raw_generation_v2(&store, &id).is_err());
    check!(eq;store.snapshot().sequence(), sequence);
    check!(!reserved.is_complete(&store)?);
    Ok(())
}

#[test]
fn equal_counters_do_not_authorize_divergent_partial_content() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let id = operation("diverged-copy")?;
    reserve_partial(&store, &id)?;
    let before = store.snapshot().public_identity_input_revisions_v2()?;
    let (key, mut value) = raw(512)?;
    value.push(b' ');
    store.entities.insert(key, value)?;
    let sequence = store.snapshot().sequence();
    check!(eq;
        store.snapshot().public_identity_input_revisions_v2()?,
        before
    );
    check!(writer.capture_raw_generation_v2(&store, &id).is_err());
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}

fn move_inputs(store: &Store) -> TestResult {
    let mut batch = store.db.batch();
    crate::input_revisions::stage(store, &mut batch, [Table::Athletes])?;
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

#[test]
fn movement_during_staging_refuses_before_seal_and_keeps_charged_partial_graph() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let id = operation("before-seal-movement")?;
    let reserved = reserve_partial(&store, &id)?;
    let held = store.snapshot();
    move_inputs(&store)?;
    check!(stage(&store, &held, &reserved).is_err());
    check!(!reserved.is_complete(&store)?);
    let sequence = store.snapshot().sequence();
    check!(writer.capture_raw_generation_v2(&store, &id).is_err());
    check!(eq;store.snapshot().sequence(), sequence);
    check!(load_graph(&store, writer.store_identity(), &id)?.is_some());
    Ok(())
}

#[test]
fn movement_after_seal_refuses_current_attempt_but_allows_historical_replay() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let id = operation("after-seal-movement")?;
    let reserved = reserve_partial(&store, &id)?;
    let held = store.snapshot();
    stage(&store, &held, &reserved)?;
    check!(reserved.is_complete(&store)?);
    move_inputs(&store)?;
    let sequence = store.snapshot().sequence();
    check!(finish_current(&store, &reserved).is_err());
    check!(eq;store.snapshot().sequence(), sequence);
    let replay = writer.capture_raw_generation_v2(&store, &id)?;
    check!(matches!(
        &replay,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    let historical = replay.retained();
    check!(eq;historical.binding(), reserved.capture());
    check!(eq;historical.seal(), reserved.seal());
    check!(eq;retained_rows(&store, historical)?, 513);
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}
