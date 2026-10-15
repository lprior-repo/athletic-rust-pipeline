use super::super::{graph_physical_rows, FrozenRawObjectSource};
use super::*;
use crate::public_identity::retained::Live;

fn physical_rows(
    store: &Store,
    retained: &RetainedRawGenerationV2,
) -> StoreResult<BTreeMap<Vec<u8>, Vec<u8>>> {
    let snapshot = store.db.snapshot();
    let source = Live {
        snapshot: &snapshot,
        entities: &store.entities,
        meta: &store.meta,
        journal: &store.journal,
        receipts: &store.receipts,
    };
    let mut rows = BTreeMap::new();
    verify::verify(&source, retained.binding(), retained.seal(), |_, object| {
        graph_physical_rows(&snapshot, &store.journal, object, |key, value| {
            if let Some(previous) = rows.insert(key.to_vec(), value.to_vec()) {
                if previous.as_slice() != value {
                    return Err(super::super::super::super::invalid(
                        "physical fixture duplicate mismatch",
                    ));
                }
            }
            Ok(())
        })
    })?;
    Ok(rows)
}

#[test]
fn real_frozen_source_reuses_original_physical_reader_after_store_close() -> TestResult {
    let (rows, retained, identity, expected) = {
        let (_dir, store, retained) = captured(513)?;
        let snapshot = store.snapshot();
        let graph = VerifiedRawGraph::verify(&snapshot, retained.binding(), retained.seal())?;
        let expected = merged(&graph)?;
        let rows = physical_rows(&store, &retained)?;
        let physical = store.db.snapshot();
        use fjall::Readable;
        for (key, value) in &rows {
            let original = physical
                .get(&store.journal, key)?
                .ok_or("physical row missing")?;
            check!(eq; value.as_slice(), original.as_ref());
        }
        (rows, retained, store.store_identity()?, expected)
    };
    let bindings = [retained.binding().clone()];
    let source = FrozenRawObjectSource::new(&identity, &bindings, &rows)?;
    let graph = VerifiedRawGraph::verify(&source, retained.binding(), retained.seal())?;
    check!(eq; merged(&graph)?, expected);
    Ok(())
}

#[test]
fn frozen_original_physical_bytes_refuse_missing_extra_and_modified_rows() -> TestResult {
    let (_dir, store, retained) = captured(1)?;
    let rows = physical_rows(&store, &retained)?;
    let identity = store.store_identity()?;
    let bindings = [retained.binding().clone()];
    let root = retained.seal().root().as_str();
    let key = crate::journal::object_chunk_key(root, 0)?;
    let mut missing = rows.clone();
    missing.remove(&key).ok_or("chunk")?;
    let source = FrozenRawObjectSource::new(&identity, &bindings, &missing)?;
    check!(matches!(
        VerifiedRawGraph::verify(&source, retained.binding(), retained.seal()),
        Err(StoreError::JournalChunkMissing { index: 0, .. })
    ));
    let mut extra = rows.clone();
    let chunk = extra.get(&key).ok_or("chunk")?.clone();
    extra.insert(crate::journal::object_chunk_key(root, 1)?, chunk);
    let source = FrozenRawObjectSource::new(&identity, &bindings, &extra)?;
    check!(matches!(
        VerifiedRawGraph::verify(&source, retained.binding(), retained.seal()),
        Err(StoreError::JournalChunkExtra { .. })
    ));
    let mut changed = rows.clone();
    changed
        .get_mut(&crate::journal::object_manifest_key(root)?)
        .ok_or("manifest")?
        .push(b' ');
    let source = FrozenRawObjectSource::new(&identity, &bindings, &changed)?;
    check!(
        matches!(VerifiedRawGraph::verify(&source, retained.binding(), retained.seal()), Err(StoreError::JournalDescriptor { detail, .. }) if detail == "retained physical bytes exceed reference")
    );
    Ok(())
}

#[test]
fn frozen_source_rejects_unlisted_and_foreign_capture_bindings() -> TestResult {
    let (_dir, store, retained) = captured(1)?;
    let rows = physical_rows(&store, &retained)?;
    let identity = store.store_identity()?;
    let source = FrozenRawObjectSource::new(&identity, &[], &rows)?;
    check!(
        matches!(VerifiedRawGraph::verify(&source, retained.binding(), retained.seal()), Err(StoreError::Refused { detail }) if detail == "raw public identity generation V2: frozen source capture identity")
    );
    let (_foreign_dir, foreign_store, _) = captured(1)?;
    let foreign_identity = foreign_store.store_identity()?;
    let bindings = [retained.binding().clone()];
    check!(
        matches!(FrozenRawObjectSource::new(&foreign_identity, &bindings, &rows), Err(StoreError::Refused { detail }) if detail == "raw public identity generation V2: frozen capture belongs to another store")
    );
    Ok(())
}

#[test]
fn physical_export_refuses_wrong_full_measurement_before_callback() -> TestResult {
    let (_dir, store, retained) = captured(1)?;
    let object = retained.seal();
    let wrong = ObjectRefV2::new(
        object.root().clone(),
        object.bytes(),
        object.records(),
        object.physical_bytes() + 1,
    )?;
    let mut called = false;
    let result = graph_physical_rows(&store.db.snapshot(), &store.journal, &wrong, |_, _| {
        called = true;
        Ok(())
    });
    check!(
        matches!(result, Err(StoreError::JournalDescriptor { detail, .. }) if detail == "retained physical bytes differ from reference")
    );
    check!(!called);
    Ok(())
}
