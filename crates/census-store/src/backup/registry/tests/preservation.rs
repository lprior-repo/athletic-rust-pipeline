use super::*;

fn backup_image(root: &Path) -> TestResult<Vec<(String, Option<String>, u64, String)>> {
    let entries = crate::backup::manifest::digest_tree(root)?;
    ensure(
        entries.len() <= FIXTURE_ROWS,
        "backup fixture file count exceeded bound",
    )?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.path, entry.target, entry.length, entry.sha256))
        .collect())
}

fn refresh_physical_manifest(root: &Path) -> TestResult {
    let path = root.join(crate::backup::MANIFEST_PATH);
    ensure(
        std::fs::metadata(&path)?.len() <= 1024 * 1024,
        "fixture manifest too large",
    )?;
    let mut manifest: crate::backup::Manifest = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut files = crate::backup::manifest::digest_tree(root)?;
    files.retain(|entry| entry.path != crate::backup::MANIFEST_PATH);
    manifest.files = files;
    crate::backup::manifest::write_manifest(root, &manifest)?;
    Ok(())
}

#[test]
fn corrupt_registry_backup_refuses_promotion_and_preserves_previous_accepted_backup() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source");
    let backup = dir.path().join("accepted-backup");
    let restored = dir.path().join("accepted-restore");
    let fixture = populate(&source, None)?;
    Store::backup(&source, &backup)?;
    let accepted = backup_image(&backup)?;
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        let id = fixture.profiles.first().ok_or("profile")?.id()?;
        inject(&store, crate::profiles::key(&id).as_bytes(), b"{")?;
    }
    match Store::backup(&source, &backup) {
        Err(StoreError::Refused { detail }) => ensure(
            detail.contains("registry metadata"),
            "backup refusal omitted metadata verification",
        )?,
        other => return Err(format!("corrupt backup was promoted={other:?}").into()),
    }
    equal(backup_image(&backup)?, accepted)?;
    Store::restore(&backup, &restored)?;
    let previous = Store::open_with(&restored, CACHE_BYTES)?;
    super::roundtrip::assert_restored(&previous, &fixture)?;
    let original = Store::open_with(&source, CACHE_BYTES)?;
    let id = fixture.profiles.first().ok_or("profile")?.id()?;
    equal(
        original
            .meta
            .get(crate::profiles::key(&id))?
            .map(|bytes| bytes.to_vec()),
        Some(b"{".to_vec()),
    )?;
    ensure(
        !original.integrity()?.ok,
        "failed backup repaired the corrupted source",
    )
}

#[test]
fn self_consistent_file_manifest_cannot_hide_corrupt_registry_during_restore_promotion(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source");
    let backup = dir.path().join("tampered-backup");
    let destination = dir.path().join("new-restore");
    let fixture = populate(&source, None)?;
    Store::backup(&source, &backup)?;
    {
        let store = Store::open_with(&backup, CACHE_BYTES)?;
        let id = fixture.profiles.first().ok_or("profile")?.id()?;
        inject(&store, crate::profiles::key(&id).as_bytes(), b"{")?;
    }
    refresh_physical_manifest(&backup)?;
    let corrupted = backup_image(&backup)?;
    match Store::restore(&backup, &destination) {
        Err(StoreError::Refused { detail }) => ensure(
            detail.contains("registry metadata"),
            "restore refusal omitted semantic registry verification",
        )?,
        other => {
            return Err(
                format!("semantically corrupt restored generation was promoted={other:?}").into(),
            )
        }
    }
    ensure(
        !destination.exists(),
        "refused restore published a destination",
    )?;
    equal(backup_image(&backup)?, corrupted)?;
    let original = Store::open_with(&source, CACHE_BYTES)?;
    super::roundtrip::assert_restored(&original, &fixture)
}

#[test]
fn unsupported_registry_backup_refuses_without_replacing_prior_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source");
    let backup = dir.path().join("accepted-backup");
    let fixture = populate(&source, None)?;
    Store::backup(&source, &backup)?;
    let accepted = backup_image(&backup)?;
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        let id = fixture.profiles.first().ok_or("profile")?.id()?;
        inject(
            &store,
            crate::profiles::key(&id).as_bytes(),
            br#"{"version":2}"#,
        )?;
    }
    match Store::backup(&source, &backup) {
        Err(StoreError::Refused { detail }) => {
            ensure(
                detail.contains("incomplete"),
                "unsupported version was not classified incomplete",
            )?;
            ensure(
                detail.contains("not a corruption diagnosis"),
                "unsupported version was mislabeled corrupt",
            )?;
        }
        other => return Err(format!("unsupported metadata was certified={other:?}").into()),
    }
    equal(backup_image(&backup)?, accepted)
}
