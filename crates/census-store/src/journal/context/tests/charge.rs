use super::*;
use fjall::Readable;

#[test]
fn physical_charge_matches_exact_encoded_rows_and_verified_ready_reuse() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let records: Vec<_> = (0..513)
        .map(|index| envelope(index, &"é".repeat(1100)))
        .collect();
    let before = store.snapshot().sequence();
    let charge = super::super::charge_context_object(&store, |sink| emit(sink, &records))?;
    assert_eq!(before, store.snapshot().sequence());
    assert_eq!(charge.roots, 1);
    let staged = stage_context_object(&store, |sink| emit(sink, &records))?;
    assert_eq!(charge.root, staged.object_id());
    let mut physical = 0u64;
    for guard in store.db.snapshot().prefix(&store.journal, b"j2\0") {
        let (key, value) = guard.into_inner()?;
        physical += u64::try_from(key.len() + value.len())?;
    }
    assert_eq!(charge.bytes, physical);
    let reused = super::super::charge_context_object(&store, |sink| emit(sink, &records))?;
    assert_eq!((reused.bytes, reused.roots), (0, 0));
    Ok(())
}

#[test]
fn partial_chunk_and_page_interruptions_keep_full_charge_and_resume_original_root() -> TestResult {
    for stop in [40, 300] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let records: Vec<_> = (0..513)
            .map(|index| envelope(index, &"x".repeat(20_000)))
            .collect();
        let charge = super::super::charge_context_object(&store, |sink| emit(sink, &records))?;
        let passes = std::cell::Cell::new(0usize);
        let before = store.snapshot().sequence();
        let result = stage_context_object(&store, |sink| {
            let pass = passes
                .get()
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            passes.set(pass);
            for (index, record) in records.iter().enumerate() {
                if pass == 2 && index == stop {
                    return Err(StoreError::Refused {
                        detail: "synthetic physical-stage interruption".into(),
                    });
                }
                sink.record(record)?;
            }
            Ok(())
        });
        assert!(result.is_err());
        assert!(store.snapshot().sequence() > before);
        assert!(store
            .db
            .snapshot()
            .get(&store.journal, object_manifest_key(&charge.root)?)?
            .is_none());
        let partial = super::super::charge_context_object(&store, |sink| emit(sink, &records))?;
        assert_eq!(charge, partial);
        store.flush()?;
        drop(store);
        let store = Store::open(directory.path())?;
        let resumed = stage_context_object(&store, |sink| emit(sink, &records))?;
        assert_eq!(resumed.object_id(), charge.root);
        assert_eq!(
            super::super::charge_context_object(&store, |sink| emit(sink, &records))?.bytes,
            0
        );
    }
    Ok(())
}
