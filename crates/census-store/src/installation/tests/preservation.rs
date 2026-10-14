use super::*;

#[test]
fn stopped_relocation_backup_and_restore_preserve_lineage_and_exact_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("original");
    let config = prepare(&root)?;
    let mut control = Store::installation_control_existing(&root)?;
    control.enroll(&config)?;
    let expected = control.show()?;
    let before = witness(&control.store)?;
    drop(control);
    let relocated = dir.path().join("relocated");
    std::fs::rename(&root, &relocated)?;
    let control = Store::installation_control_existing(&relocated)?;
    equal(control.show()?, expected.clone())?;
    equal(witness(&control.store)?, before.clone())?;
    drop(control);
    let backup = dir.path().join("backup");
    Store::backup(&relocated, &backup)?;
    let restored = dir.path().join("restored");
    Store::restore(&backup, &restored)?;
    let control = Store::installation_control_existing(&restored)?;
    equal(control.show()?, expected)?;
    equal(witness(&control.store)?.rows, before.rows)
}

#[test]
fn malformed_enrollment_cannot_promote_a_backup_over_a_valid_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let config = prepare(&root)?;
    let mut control = Store::installation_control_existing(&root)?;
    control.enroll(&config)?;
    drop(control);
    let backup = dir.path().join("backup");
    Store::backup(&root, &backup)?;
    let manifest_before = std::fs::read(backup.join("backup.json"))?;
    let control = Store::installation_control_existing(&root)?;
    control
        .store
        .meta
        .remove(super::super::super::ENROLLMENT_KEY)?;
    control.store.flush()?;
    drop(control);
    if Store::backup(&root, &backup).is_ok() {
        return Err("orphan backup promoted".into());
    }
    equal(std::fs::read(backup.join("backup.json"))?, manifest_before)?;
    let restored = dir.path().join("restored");
    Store::restore(&backup, &restored)?;
    let control = Store::installation_control_existing(&restored)?;
    match control.show()? {
        InstallationState::Enrolled {
            config: retained, ..
        } => equal(retained, config),
        other => Err(format!("retained backup={other:?}").into()),
    }
}

#[test]
fn non_enrolled_schema_one_backup_restore_preserves_the_preexisting_raw_copy_policy() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("historical");
    let store = Store::open_with(&root, 8_388_608)?;
    store.append(
        Table::Schools,
        &serde_json::json!({"id":"retained","name":"historical"}),
    )?;
    store.meta.insert("schema:store_version", b"1")?;
    store.flush()?;
    let before = witness(&store)?;
    drop(store);
    let backup = dir.path().join("backup");
    let report = Store::backup(&root, &backup)?;
    equal(report.tables.get(Table::Schools.file()).copied(), Some(1))?;
    let restored = dir.path().join("restored");
    Store::restore(&backup, &restored)?;
    let (db, entities, journal, meta, receipts) = crate::open_keyspaces(&restored, 8_388_608)?;
    equal(crate::format::read_format(&meta)?.schema_version, Some(1))?;
    equal(
        vec![
            rows(&db.snapshot(), &entities)?,
            rows(&db.snapshot(), &journal)?,
            rows(&db.snapshot(), &meta)?,
            rows(&db.snapshot(), &receipts)?,
        ],
        before.rows,
    )
}

fn candidate_files(root: &std::path::Path) -> TestResult<Vec<crate::backup::ManifestEntry>> {
    use sha2::{Digest, Sha256};
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
            let relative = path.strip_prefix(root)?.to_string_lossy().to_string();
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
                length: u64::try_from(bytes.len())?,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            });
        }
    }
    Err("candidate directory budget".into())
}

#[test]
fn malformed_restore_candidate_passes_file_hashes_but_cannot_promote_into_empty_destination(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source");
    let config = prepare(&source)?;
    let mut control = Store::installation_control_existing(&source)?;
    control.enroll(&config)?;
    drop(control);
    let backup = dir.path().join("backup");
    Store::backup(&source, &backup)?;
    let control = Store::installation_control_existing(&backup)?;
    control
        .store
        .meta
        .remove(super::super::super::ENROLLMENT_KEY)?;
    control.store.flush()?;
    drop(control);
    let manifest_path = backup.join("backup.json");
    let mut manifest: crate::backup::Manifest =
        serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest.files = candidate_files(&backup)?;
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    let destination = dir.path().join("destination");
    std::fs::create_dir(&destination)?;
    let before = std::fs::symlink_metadata(&destination)?;
    match Store::restore(&backup, &destination) {
        Err(error) if error.to_string().contains("enrollment") => {}
        other => return Err(format!("malformed restore candidate={other:?}").into()),
    }
    equal(destination.is_dir(), true)?;
    equal(std::fs::read_dir(&destination)?.next().is_none(), true)?;
    equal(
        std::fs::symlink_metadata(&destination)?.modified()?,
        before.modified()?,
    )
}

#[test]
fn whitespace_lifecycle_three_without_enrollment_refuses_backup_and_preserves_prior_generation(
) -> TestResult {
    for marker in ["3 ", " 3", "\t3\n"] {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("store");
        let _config = prepare(&root)?;
        let backup = dir.path().join("backup");
        Store::backup(&root, &backup)?;
        let prior_manifest = std::fs::read(backup.join("backup.json"))?;
        let store = Store::open_existing(&root)?;
        store
            .meta
            .insert(crate::format::LIFECYCLE_REVISION_KEY, marker)?;
        store.flush()?;
        let before = witness(&store)?;
        drop(store);
        match Store::backup(&root, &backup) {
            Err(error) if error.to_string().contains("enrollment") => {}
            other => return Err(format!("whitespace marker backup={other:?}").into()),
        }
        equal(std::fs::read(backup.join("backup.json"))?, prior_manifest)?;
        let (db, entities, journal, meta, receipts) = crate::open_keyspaces(&root, 8_388_608)?;
        equal(
            vec![
                rows(&db.snapshot(), &entities)?,
                rows(&db.snapshot(), &journal)?,
                rows(&db.snapshot(), &meta)?,
                rows(&db.snapshot(), &receipts)?,
            ],
            before.rows,
        )?;
    }
    Ok(())
}

#[test]
fn whitespace_lifecycle_three_candidate_refuses_restore_before_empty_destination_promotion(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let _config = prepare(&root)?;
    let backup = dir.path().join("backup");
    Store::backup(&root, &backup)?;
    {
        let (db, entities, journal, meta, receipts) = crate::open_keyspaces(&backup, 8_388_608)?;
        meta.insert(crate::format::LIFECYCLE_REVISION_KEY, b" 3 ")?;
        db.persist(fjall::PersistMode::SyncAll)?;
        drop((entities, journal, meta, receipts));
    }
    let manifest_path = backup.join("backup.json");
    let mut manifest: crate::backup::Manifest =
        serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest.files = candidate_files(&backup)?;
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    let destination = dir.path().join("destination");
    std::fs::create_dir(&destination)?;
    match Store::restore(&backup, &destination) {
        Err(error) if error.to_string().contains("enrollment") => {}
        other => return Err(format!("whitespace marker restore={other:?}").into()),
    }
    equal(std::fs::read_dir(&destination)?.next().is_none(), true)
}
