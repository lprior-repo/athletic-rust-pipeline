use super::*;

#[test]
fn lifecycle_three_requires_complete_enrollment_and_refuses_every_orphan() -> TestResult {
    for orphan in ["marker", "enrollment", "config"] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let mut control = Store::installation_control_existing(root.path())?;
        control.enroll(&config)?;
        match orphan {
            "marker" => {
                control
                    .store
                    .meta
                    .remove(super::super::super::ENROLLMENT_KEY)?;
                control.store.meta.remove(format!(
                    "{}{}",
                    super::super::super::CONFIG_PREFIX,
                    config.id()?
                ))?;
            }
            "enrollment" => {
                control
                    .store
                    .meta
                    .remove(super::super::super::ENROLLMENT_KEY)?;
            }
            "config" => {
                control.store.meta.remove(format!(
                    "{}{}",
                    super::super::super::CONFIG_PREFIX,
                    config.id()?
                ))?;
            }
            _ => return Err("orphan fixture".into()),
        }
        control.store.flush()?;
        let before = witness(&control.store)?;
        drop(control);
        let expected = match orphan {
            "marker" | "enrollment" => "enrollment",
            "config" => "config",
            _ => return Err("orphan".into()),
        };
        for result in [
            Store::open_with(root.path(), 8_388_608).map(|_| ()),
            Store::open_existing(root.path()).map(|_| ()),
            Store::installation_control_existing(root.path()).map(|_| ()),
            Store::inspect_with(root.path(), 8_388_608).map(|_| ()),
            Store::migrate_with(root.path(), 8_388_608).map(|_| ()),
        ] {
            match (result, expected) {
                (
                    Err(StoreError::Installation(InstallationError::MissingEnrollment)),
                    "enrollment",
                ) => {}
                (Err(StoreError::Installation(InstallationError::MissingConfig)), "config") => {}
                (other, _) => return Err(format!("{orphan} refusal={other:?}").into()),
            }
        }
        let (db, entities, journal, meta, receipts) =
            crate::open_keyspaces(root.path(), 8_388_608)?;
        let after = Witness {
            sequence: db.snapshot().seqno(),
            evidence: 0,
            derived: 0,
            rows: vec![
                rows(&db.snapshot(), &entities)?,
                rows(&db.snapshot(), &journal)?,
                rows(&db.snapshot(), &meta)?,
                rows(&db.snapshot(), &receipts)?,
            ],
        };
        equal(after, before)?;
    }
    Ok(())
}

#[test]
fn enrollment_records_with_legacy_marker_are_not_silently_upgraded() -> TestResult {
    for marker in [None, Some(0), Some(1), Some(2)] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let mut control = Store::installation_control_existing(root.path())?;
        control.enroll(&config)?;
        match marker {
            None => {
                control
                    .store
                    .meta
                    .remove(crate::format::LIFECYCLE_REVISION_KEY)?;
            }
            Some(marker) => {
                control
                    .store
                    .meta
                    .insert(crate::format::LIFECYCLE_REVISION_KEY, marker.to_string())?;
            }
        }
        control.store.flush()?;
        drop(control);
        for result in [
            Store::open(root.path()).map(|_| ()),
            Store::inspect(root.path()).map(|_| ()),
            Store::migrate(root.path()).map(|_| ()),
        ] {
            if !matches!(result, Err(StoreError::Installation(InstallationError::MarkerMismatch { found })) if found == marker)
            {
                return Err(format!("legacy marker {marker:?} was not refused").into());
            }
        }
    }
    Ok(())
}

#[test]
fn newer_format_wins_before_decoding_invalid_enrollment() -> TestResult {
    for (key, value, what, supported) in [
        ("schema:lifecycle_revision", "4", "lifecycle revision", 3),
        ("schema:store_version", "3", "schema version", 2),
        ("schema:key_format", "2", "key format", 1),
    ] {
        let root = tempfile::tempdir()?;
        let _config = prepare(root.path())?;
        let store = Store::open_existing(root.path())?;
        store
            .meta
            .insert(super::super::super::ENROLLMENT_KEY, b"{")?;
        store.meta.insert(key, value)?;
        store.flush()?;
        drop(store);
        match Store::installation_control_existing(root.path()) {
            Err(StoreError::FormatNewer {
                what: actual,
                found,
                supported: actual_supported,
            }) => {
                equal(actual, what)?;
                equal(found.to_string(), value.to_string())?;
                equal(actual_supported, supported)?;
            }
            Err(other) => return Err(format!("newer format={other:?}").into()),
            Ok(_) => return Err("newer format accepted".into()),
        }
    }
    Ok(())
}

#[test]
fn corrupt_digest_foreign_lineage_unknown_namespace_and_multiple_rows_refuse() -> TestResult {
    for corruption in [
        "digest",
        "lineage",
        "namespace",
        "multiple",
        "unknown_fields",
        "future_version",
    ] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let mut control = Store::installation_control_existing(root.path())?;
        control.enroll(&config)?;
        let config_key = format!("{}{}", super::super::super::CONFIG_PREFIX, config.id()?);
        match corruption {
            "digest" => {
                let mut value: serde_json::Value =
                    serde_json::from_slice(&control.store.meta.get(&config_key)?.ok_or("config")?)?;
                value.get_mut("payload").and_then(serde_json::Value::as_object_mut).ok_or("payload")?.insert("service_resources".to_string(), serde_json::json!({"max_concurrent":9,"memory_budget_bytes":48 * census_domain::model::ServiceResourceProfileV1::GIB,"drain_timeout_seconds":30}));
                control
                    .store
                    .meta
                    .insert(&config_key, serde_json::to_vec(&value)?)?;
            }
            "lineage" => {
                control.store.journal_done(
                    "export-lineage-v1",
                    "store-identity",
                    &census_domain::model::StoreIdentity::parse(&"f".repeat(64))?,
                )?;
            }
            "namespace" => {
                control
                    .store
                    .meta
                    .insert(b"installation:v2:enrollment", b"{")?;
            }
            "multiple" => {
                control.store.meta.insert(
                    format!("{}{}", super::super::super::CONFIG_PREFIX, "e".repeat(64)),
                    b"{",
                )?;
            }
            "unknown_fields" => {
                let mut value: serde_json::Value = serde_json::from_slice(
                    &control
                        .store
                        .meta
                        .get(super::super::super::ENROLLMENT_KEY)?
                        .ok_or("enrollment")?,
                )?;
                value
                    .as_object_mut()
                    .ok_or("enrollment")?
                    .insert("extra".to_string(), serde_json::json!(true));
                control.store.meta.insert(
                    super::super::super::ENROLLMENT_KEY,
                    serde_json::to_vec(&value)?,
                )?;
            }
            "future_version" => {
                control
                    .store
                    .meta
                    .insert(super::super::super::ENROLLMENT_KEY, br#"{"version":2}"#)?;
            }
            _ => return Err("corruption".into()),
        }
        control.store.flush()?;
        drop(control);
        let error = match Store::installation_control_existing(root.path()) {
            Err(error) => error,
            Ok(_) => return Err("corruption accepted".into()),
        };
        match (corruption, error) {
            ("digest", StoreError::Installation(InstallationError::ConfigIdMismatch))
            | ("lineage", StoreError::Installation(InstallationError::ForeignStore))
            | ("namespace", StoreError::Installation(InstallationError::UnsupportedNamespace))
            | ("multiple", StoreError::Installation(InstallationError::MultipleRecords))
            | ("unknown_fields", StoreError::Decode { .. })
            | (
                "future_version",
                StoreError::Installation(InstallationError::UnsupportedVersion {
                    found: 2, ..
                }),
            ) => {}
            (kind, other) => return Err(format!("{kind} refusal={other:?}").into()),
        }
    }
    Ok(())
}
