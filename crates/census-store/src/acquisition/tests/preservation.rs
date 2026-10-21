use super::*;

#[test]
fn stopped_relocation_backup_restore_preserves_identity_attempts_receipts_and_cache_v2(
) -> TestResult {
    let (dir, store, writer, owner) = fixture()?;
    let occurrence = retained(&store, &writer, &owner, 200)?;
    let verified =
        store.read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())?;
    let metadata: serde_json::Value = serde_json::from_slice(verified.prepared().metadata_bytes())?;
    let envelope = serde_json::to_vec_pretty(
        &serde_json::json!({"version":2,"metadata":metadata,"original_occurrence":occurrence}),
    )?;
    std::fs::write(store.http_cache_dir().join("fixture.meta.json"), &envelope)?;
    std::fs::write(
        store.http_cache_dir().join("fixture.body"),
        std::fs::read(verified.body_path())?,
    )?;
    let identity = writer.store_identity().clone();
    let before = rows(&store)?;
    drop(store);
    let relocated = dir.path().join("relocated");
    std::fs::rename(dir.path().join("store"), &relocated)?;
    let backup = dir.path().join("backup");
    Store::backup(&relocated, &backup)?;
    let restored = dir.path().join("restored");
    Store::restore(&backup, &restored)?;
    let (store, writer) = Store::open_current_qualification_v3(&restored)?.into_parts();
    check!(eq; writer.store_identity(), &identity);
    check!(eq; rows(&store)?, before);
    check!(eq; std::fs::read(store.http_cache_dir().join("fixture.meta.json"))?, envelope);
    check!(eq; store.read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())?.reference(), &occurrence);
    Ok(())
}

#[test]
fn invalid_capture_backup_cannot_replace_previous_valid_generation() -> TestResult {
    for damage in [
        "missing-marker",
        "missing-backing",
        "orphan-root",
        "foreign-cache",
        "unsupported-cache",
    ] {
        let (dir, store, writer, owner) = fixture()?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        drop(store);
        let root = dir.path().join("store");
        let backup = dir.path().join("backup");
        Store::backup(&root, &backup)?;
        let manifest = std::fs::read(backup.join("backup.json"))?;
        let (store, _) = Store::open_current_qualification_v3(&root)?.into_parts();
        match damage {
            "missing-marker" => {
                store
                    .meta
                    .remove(CaptureProtocolVersion::V2.qualification_key())?;
            }
            "missing-backing" => {
                let verified = store.read_retained_acquisition_v1(
                    &occurrence,
                    &mut AcquisitionReadBudget::for_work(),
                )?;
                std::fs::remove_file(verified.body_path())?;
            }
            "orphan-root" => overwrite(
                &store,
                &format!("work/{}/identity", "b".repeat(64)),
                &"orphan",
            )?,
            "foreign-cache" | "unsupported-cache" => {
                let reference = AcquisitionOccurrenceRefV1::new(
                    AcquisitionAttemptId::new(
                        StoreIdentity::parse(&"F".repeat(64))?,
                        occurrence.attempt().work_id().clone(),
                        AttemptOrdinal::new(1)?,
                    ),
                    occurrence.terminal_digest().clone(),
                );
                std::fs::write(
                    store.http_cache_dir().join("invalid.meta.json"),
                    serde_json::to_vec(
                        &serde_json::json!({"version":if damage == "unsupported-cache" { 3 } else { 2 },"metadata":{},"original_occurrence":reference}),
                    )?,
                )?;
            }
            _ => return Err("damage".into()),
        }
        store.flush()?;
        drop(store);
        check!(Store::backup(&root, &backup).is_err());
        check!(eq; std::fs::read(backup.join("backup.json"))?, manifest);
        let restored = dir.path().join("restored");
        Store::restore(&backup, &restored)?;
        let (restored, _) = Store::open_current_qualification_v3(&restored)?.into_parts();
        check!(restored
            .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())
            .is_ok());
    }
    Ok(())
}

#[test]
fn historical_non_capture_backup_keeps_raw_schema_preservation_policy() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("legacy");
    let store = Store::open_with(&root, 8_388_608)?;
    store.store_identity()?;
    store
        .meta
        .insert("schema:store_version", b"historical-opaque")?;
    store.flush()?;
    let before = rows(&store)?;
    drop(store);
    let backup = dir.path().join("backup");
    Store::backup(&root, &backup)?;
    let restored = dir.path().join("restored");
    Store::restore(&backup, &restored)?;
    let (db, _, journal, meta, receipts) = crate::open_keyspaces(&restored, 8_388_608)?;
    let mut after = Vec::new();
    for keyspace in [&meta, &journal, &receipts] {
        for row in db.snapshot().prefix(keyspace, b"").take(1024) {
            let (key, value) = row.into_inner()?;
            after.push((key.to_vec(), value.to_vec()));
        }
    }
    check!(eq; before, after);
    Ok(())
}

#[test]
fn capture_control_preserves_uppercase_existing_identity_and_never_remints_missing_identity(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    Store::create_capture_qualification(&root)?;
    let (store, _) = Store::open_current_qualification_v3(&root)?.into_parts();
    let identity = StoreIdentity::parse(&"Ab".repeat(32))?;
    let key = Store::journal_key("export-lineage-v1", "store-identity");
    store.journal.insert(
        &key,
        serde_json::to_vec(
            &serde_json::json!({"key":"store-identity","at":"2026-10-14","payload":identity}),
        )?,
    )?;
    store.meta.insert(
        CaptureProtocolVersion::V2.qualification_key(),
        serialized_bytes(&CaptureQualificationV2::new(identity.clone()))?,
    )?;
    store.flush()?;
    drop(store);
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    check!(eq; writer.store_identity(), &identity);
    store.journal.remove(&key)?;
    check!(store.store_identity().is_err());
    check!(!store.journal.contains_key(&key)?);
    Ok(())
}

#[test]
fn a_hash_consistent_invalid_restore_cannot_promote_over_an_empty_destination() -> TestResult {
    let (dir, store, writer, owner) = fixture()?;
    retained(&store, &writer, &owner, 200)?;
    drop(store);
    let backup = dir.path().join("backup");
    Store::backup(&dir.path().join("store"), &backup)?;
    let (store, _) = Store::open_current_qualification_v3(&backup)?.into_parts();
    store
        .meta
        .remove(CaptureProtocolVersion::V2.qualification_key())?;
    store.flush()?;
    drop(store);
    let path = backup.join("backup.json");
    let mut manifest: crate::backup::Manifest = serde_json::from_slice(&std::fs::read(&path)?)?;
    manifest.files = candidate_files(&backup)?;
    std::fs::write(&path, serde_json::to_vec(&manifest)?)?;
    let destination = dir.path().join("destination");
    std::fs::create_dir(&destination)?;
    let before = std::fs::metadata(&destination)?.modified()?;
    check!(Store::restore(&backup, &destination).is_err());
    check!(std::fs::read_dir(&destination)?.next().is_none());
    check!(eq; std::fs::metadata(&destination)?.modified()?, before);
    Ok(())
}

fn candidate_files(root: &std::path::Path) -> TestResult<Vec<crate::backup::ManifestEntry>> {
    let mut directories = vec![root.to_path_buf()];
    let mut files = Vec::new();
    for _ in 0..64 {
        let Some(directory) = directories.pop() else {
            files.sort_by(|left: &crate::backup::ManifestEntry, right| left.path.cmp(&right.path));
            return Ok(files);
        };
        for entry in std::fs::read_dir(directory)?.take(513) {
            if files.len() == 512 {
                return Err("candidate file budget".into());
            }
            let path = entry?.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                directories.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            if relative == "backup.json" {
                continue;
            }
            if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
                return Err("candidate type/size budget".into());
            }
            let bytes = std::fs::read(&path)?;
            files.push(crate::backup::ManifestEntry {
                path: relative,
                target: None,
                length: metadata.len(),
                sha256: format!("{:x}", Sha256::digest(bytes)),
            });
        }
    }
    Err("candidate directory budget".into())
}
