use super::*;

#[test]
fn ready_manifest_removed_during_callback_is_refused_without_recreation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = [envelope(0, "original")];
    let staged = stage_context_object(&store, |sink| emit(sink, &records))?;
    let manifest_key = object_manifest_key(staged.object_id())?;
    let chunk_key = object_chunk_key(staged.object_id(), 0)?;
    let index_key = object_index_key(staged.object_id(), 0)?;
    let chunk_before = store.journal.get(&chunk_key)?.ok_or("original chunk")?;
    let index_before = store.journal.get(&index_key)?.ok_or("original index")?;
    let pass = std::cell::Cell::new(0);
    let removed_sequence = std::cell::Cell::new(None);
    let result = stage_context_object(&store, |sink| {
        pass.set(pass.get() + 1);
        emit(sink, &records)?;
        if pass.get() == 2 {
            let mut batch = store.db.batch();
            batch.remove(&store.journal, &manifest_key);
            batch
                .commit()
                .map_err(|source| StoreError::Write { source })?;
            removed_sequence.set(Some(store.snapshot().sequence()));
        }
        Ok(())
    });
    match result {
        Err(StoreError::Refused { detail }) => check!(eq; detail,
            format!("journal context object: ready object has missing physical row {manifest_key:?}")),
        other => {
            return Err(format!("expected missing ready manifest refusal, got {other:?}").into())
        }
    }
    check!(eq; pass.get(), 2);
    check!(eq; removed_sequence.get(), Some(store.snapshot().sequence()));
    check!(store.journal.get(&manifest_key)?.is_none());
    check!(eq; store.journal.get(&chunk_key)?.ok_or("retained chunk")?, chunk_before);
    check!(eq; store.journal.get(&index_key)?.ok_or("retained index")?, index_before);
    check!(eq; store.journal.len()?, 2);
    Ok(())
}

#[test]
fn staged_rows_are_revalidated_after_callback_before_manifest_publication() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = [envelope(0, &"x".repeat(600_000))];
    let expected = plan(&records)?;
    let key = object_chunk_key(expected.staged.object_id(), 0)?;
    let pass = std::cell::Cell::new(0);
    refused(
        stage_context_object(&store, |sink| {
            pass.set(pass.get() + 1);
            emit(sink, &records)?;
            if pass.get() == 2 {
                let mut batch = store.db.batch();
                batch.insert(&store.journal, &key, b"tampered after staging");
                batch
                    .commit()
                    .map_err(|source| StoreError::Write { source })?;
            }
            Ok(())
        }),
        "staged physical row changed",
    )?;
    check!(store
        .journal
        .get(object_manifest_key(expected.staged.object_id())?)?
        .is_none());
    check!(eq; store.journal.len()?, 3);
    Ok(())
}

#[test]
fn unstable_callback_stages_only_verified_chunks_and_can_resume() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = [envelope(0, &"x".repeat(600_000)), envelope(1, "first")];
    let expected = plan(&records)?;
    let pass = std::cell::Cell::new(0);
    refused(
        stage_context_object(&store, |sink| {
            pass.set(pass.get() + 1);
            sink.record(
                records
                    .first()
                    .ok_or_else(|| super::super::refused("missing test record"))?,
            )?;
            let tail = envelope(1, if pass.get() == 1 { "first" } else { "other" });
            sink.record(&tail)
        }),
        "callback changed chunk 1",
    )?;
    check!(eq; store.journal.len()?, 1);
    check!(store
        .journal
        .get(object_manifest_key(expected.staged.object_id())?)?
        .is_none());
    let staged = stage_context_object(&store, |sink| emit(sink, &records))?;
    check!(eq; staged, expected.staged);
    check!(eq; store.journal.len()?, 4);
    Ok(())
}

#[test]
fn shorter_second_pass_cannot_publish_partial_context() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let exact = "x".repeat(super::super::CONTEXT_RECORD_BYTES - 3);
    let first_pass = run_pass(Pass::Measure, &|sink| {
        sink.record(&exact)?;
        sink.record(&"tail")
    })?;
    let pass = std::cell::Cell::new(0);
    refused(
        stage_context_object(&store, |sink| {
            pass.set(pass.get() + 1);
            sink.record(&exact)?;
            if pass.get() == 1 {
                sink.record(&"tail")?;
            }
            Ok(())
        }),
        "callback changed index page 0",
    )?;
    check!(eq; store.journal.len()?, 2);
    check!(store
        .journal
        .get(object_manifest_key(first_pass.staged.object_id())?)?
        .is_none());
    Ok(())
}

#[test]
fn callback_failure_leaves_manifest_absent_after_durable_chunk_staging() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = [envelope(0, &"x".repeat(600_000))];
    let expected = plan(&records)?;
    let pass = std::cell::Cell::new(0);
    refused(
        stage_context_object(&store, |sink| {
            pass.set(pass.get() + 1);
            emit(sink, &records)?;
            if pass.get() == 2 {
                return Err(StoreError::Refused {
                    detail: "callback interrupted".to_string(),
                });
            }
            Ok(())
        }),
        "callback interrupted",
    )?;
    check!(eq; store.journal.len()?, 1);
    drop(store);
    let store = Store::open(dir.path())?;
    check!(store
        .journal
        .get(object_manifest_key(expected.staged.object_id())?)?
        .is_none());
    check!(eq; store.journal.len()?, 1);
    Ok(())
}

#[test]
fn conflicting_chunk_index_and_manifest_bytes_are_never_overwritten() -> TestResult {
    let records = [envelope(0, "immutable")];
    let expected = plan(&records)?;
    let object = expected.staged.object_id();
    let keys = [
        object_chunk_key(object, 0)?,
        object_index_key(object, 0)?,
        object_manifest_key(object)?,
    ];
    keys.into_iter().try_for_each(|key| {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        write_row(&store, &key, b"conflicting")?;
        refused(stage_context_object(&store, |sink| emit(sink, &records)), "immutable physical row conflicts")?;
        check!(eq; store.journal.get(&key)?.ok_or("conflicting row lost")?.as_ref(), b"conflicting");
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

#[test]
fn ready_manifest_does_not_bypass_validation_of_corrupt_or_missing_chunks() -> TestResult {
    [false, true].into_iter().try_for_each(|remove| {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let records = [envelope(0, "original")];
        let staged = stage_context_object(&store, |sink| emit(sink, &records))?;
        let key = object_chunk_key(staged.object_id(), 0)?;
        let mut batch = store.db.batch();
        if remove {
            batch.remove(&store.journal, &key);
        } else {
            batch.insert(&store.journal, &key, b"corrupt");
        }
        batch.commit()?;
        let sequence = store.snapshot().sequence();
        let message = if remove {
            "ready object has missing physical row"
        } else {
            "immutable physical row conflicts"
        };
        refused(
            stage_context_object(&store, |sink| emit(sink, &records)),
            message,
        )?;
        check!(eq; store.snapshot().sequence(), sequence);
        check!(eq; store.journal.get(&key)?.is_none(), remove);
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

#[test]
fn ready_manifest_does_not_bypass_index_or_namespace_validation() -> TestResult {
    let records = [envelope(0, "original")];
    let expected = plan(&records)?;
    let object = expected.staged.object_id();
    let keys = [
        object_index_key(object, 0)?,
        object_chunk_key(object, 1)?,
        object_index_key(object, 1)?,
    ];
    keys.into_iter().enumerate().try_for_each(|(index, key)| {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        stage_context_object(&store, |sink| emit(sink, &records))?;
        write_row(&store, &key, b"corrupt")?;
        let sequence = store.snapshot().sequence();
        let message = if index == 0 {
            "immutable physical row conflicts"
        } else {
            "extra or malformed physical row"
        };
        refused(
            stage_context_object(&store, |sink| emit(sink, &records)),
            message,
        )?;
        check!(eq; store.snapshot().sequence(), sequence);
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

#[test]
fn existing_physical_row_size_is_checked_before_loading_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = [envelope(0, "original")];
    let expected = plan(&records)?;
    let key = object_chunk_key(expected.staged.object_id(), 0)?;
    write_row(
        &store,
        &key,
        &vec![b'x'; crate::MAX_JOURNAL_VALUE_BYTES + 1],
    )?;
    match stage_context_object(&store, |sink| emit(sink, &records)) {
        Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        }) => {
            check!(eq; what, "context physical row bytes");
            check!(eq; needed, u64::try_from(crate::MAX_JOURNAL_VALUE_BYTES + 1)?);
            check!(eq; available, u64::try_from(crate::MAX_JOURNAL_VALUE_BYTES)?);
        }
        other => return Err(format!("expected physical-row size refusal, got {other:?}").into()),
    }
    check!(store
        .journal
        .get(object_manifest_key(expected.staged.object_id())?)?
        .is_none());
    Ok(())
}
