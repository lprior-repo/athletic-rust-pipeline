use super::*;

#[test]
fn interrupted_descendant_resumes_after_store_reopen_without_second_reservation() -> TestResult {
    let (directory, store, writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 800_000)?;
    let reservation = reserve(&store, "interrupted-graph", plan)?;
    let root = reservation
        .descriptors
        .first()
        .ok_or("page descriptor")?
        .object
        .root()
        .as_str()
        .to_owned();
    let charged = counters(&store)?;
    let header = header_bytes(&store)?;
    let encoded = generation.page.encode_jsonl()?;
    let records = std::str::from_utf8(&encoded)?
        .lines()
        .map(serde_json::from_str::<&serde_json::value::RawValue>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(records.len(), 4);
    let permit = reservation.permit_for(&store, &root)?;
    let passes = std::cell::Cell::new(0usize);
    let result = crate::journal::context::stage_reserved_context(&store, &permit, |sink| {
        let pass = passes
            .get()
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        passes.set(pass);
        for (index, record) in records.iter().enumerate() {
            if pass == 2 && index == 3 {
                return Err(StoreError::Refused {
                    detail: "graph fixture descendant interruption".into(),
                });
            }
            sink.record(record)?;
        }
        Ok(())
    });
    assert!(
        matches!(result, Err(StoreError::Refused { detail }) if detail == "graph fixture descendant interruption")
    );
    assert_eq!(passes.get(), 2);
    assert_eq!(physical_rows(&store)?.len(), 1);
    let chunk_key = crate::journal::object_chunk_key(&root, 0)?;
    let retained_chunk = row(&store, &chunk_key)?.ok_or("interrupted chunk")?;
    assert!(row(&store, &crate::journal::object_manifest_key(&root)?)?.is_none());
    assert!(row(&store, &crate::journal::object_index_key(&root, 0)?)?.is_none());
    assert!(!reservation.is_complete(&store)?);
    assert_eq!(counters(&store)?, charged);
    assert_eq!(header_bytes(&store)?, header);
    reservation.require_plan(&generation.plan(&store)?)?;
    drop(permit);
    store.flush()?;
    drop(writer);
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let reservation = load(&store, "interrupted-graph")?.ok_or("reopened reservation")?;
    let before_retry = retained_state(&store)?;
    let plan = generation.plan(&store)?;
    reservation.require_plan(&plan)?;
    assert_eq!(retained_state(&store)?, before_retry);
    assert_eq!(
        plan.descriptors
            .first()
            .ok_or("page descriptor")?
            .incremental
            .roots,
        1
    );
    generation.stage(&store, &reservation)?;
    assert!(reservation.is_complete(&store)?);
    assert_eq!(row(&store, &chunk_key)?, Some(retained_chunk));
    assert_eq!(counters(&store)?, charged);
    assert_eq!(header_bytes(&store)?, header);
    let completed = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    generation.stage(&store, &reservation)?;
    assert_eq!(retained_state(&store)?, completed);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn recovered_partial_descendant_keeps_original_full_charge_and_exact_rows() -> TestResult {
    let (directory, store, writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "recovered-partial", plan)?;
    stage(&store, &reservation, bridge::Node::Page(&generation.page))?;
    let root = reservation
        .descriptors
        .first()
        .ok_or("page descriptor")?
        .object
        .root()
        .as_str();
    let chunk_key = crate::journal::object_chunk_key(root, 0)?;
    let chunk = row(&store, &chunk_key)?.ok_or("staged page chunk")?;
    remove(
        &store,
        &[
            crate::journal::object_manifest_key(root)?,
            crate::journal::object_index_key(root, 0)?,
        ],
    )?;
    assert_eq!(physical_rows(&store)?.len(), 1);
    assert!(!reservation.is_complete(&store)?);
    let charged = counters(&store)?;
    store.flush()?;
    drop(writer);
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let reservation = load(&store, "recovered-partial")?.ok_or("recovered reservation")?;
    let current = generation.plan(&store)?;
    reservation.require_plan(&current)?;
    assert_eq!(
        current
            .descriptors
            .first()
            .ok_or("current page")?
            .incremental,
        reservation
            .descriptors
            .first()
            .ok_or("reserved page")?
            .incremental
    );
    generation.stage(&store, &reservation)?;
    assert_eq!(row(&store, &chunk_key)?, Some(chunk));
    assert_eq!(counters(&store)?, charged);
    assert!(reservation.is_complete(&store)?);
    Ok(())
}
