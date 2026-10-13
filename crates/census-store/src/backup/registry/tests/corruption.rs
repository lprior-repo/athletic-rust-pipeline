use super::*;

fn metadata_report(store: &Store) -> TestResult<Report> {
    Ok(check(
        &store.db.snapshot(),
        &store.meta,
        MAX_RECORDS,
        MAX_BYTES,
    )?)
}

fn assert_class(store: &Store, class: CorruptionClass) -> TestResult {
    let before = Witness::capture(store)?;
    let report = metadata_report(store)?;
    ensure(
        report
            .corruptions
            .iter()
            .any(|finding| finding.class == class),
        "expected metadata corruption class absent",
    )?;
    ensure(
        !store.integrity()?.ok,
        "generic integrity omitted registry corruption",
    )?;
    equal(Witness::capture(store)?, before)
}

#[test]
fn malformed_profile_is_registry_corruption_while_entity_counts_stay_equal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fixture = populate(dir.path(), None)?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let profile = fixture.profiles.first().ok_or("profile")?;
    inject(
        &store,
        crate::profiles::key(&profile.id()?).as_bytes(),
        b"{",
    )?;
    equal(store.count(Table::Schools)?, 1)?;
    equal(
        store.snapshot().tables_digest(&Table::ALL)?,
        fixture.witness.table_digest,
    )?;
    assert_class(&store, CorruptionClass::Undecodable)
}

#[test]
fn missing_region_chunk_is_truncated_and_registration_does_not_repair_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fixture = populate(dir.path(), None)?;
    let region = fixture.regions.first().ok_or("region")?;
    let reference = region.reference()?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    remove(&store, &chunk(&reference, 0)?)?;
    assert_class(&store, CorruptionClass::Truncated)?;
    let before = Witness::capture(&store)?;
    match store.register_region_contract(&reference, region) {
        Err(StoreError::RegionContractRegistry(
            crate::RegionContractRegistryError::MissingPayload {
                reference: missing,
                index,
            },
        )) => {
            equal(missing, reference)?;
            equal(index, 0)?;
        }
        other => return Err(format!("missing chunk repair={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn region_chunks_without_descriptor_are_orphan_truncation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fixture = populate(dir.path(), None)?;
    let reference = fixture.regions.first().ok_or("region")?.reference()?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    remove(&store, descriptor(&reference).as_bytes())?;
    assert_class(&store, CorruptionClass::Truncated)
}

#[test]
fn extra_and_malformed_regional_chunk_keys_are_undecodable() -> TestResult {
    for malformed in [false, true] {
        let dir = tempfile::tempdir()?;
        let fixture = populate(dir.path(), None)?;
        let reference = fixture.regions.first().ok_or("region")?.reference()?;
        let store = Store::open_with(dir.path(), CACHE_BYTES)?;
        let mut key = chunk(&reference, 1)?;
        if malformed {
            key.push(0);
        }
        inject(&store, &key, b"retained-extra")?;
        assert_class(&store, CorruptionClass::Undecodable)?;
    }
    Ok(())
}

#[test]
fn same_length_changed_region_payload_is_a_digest_binding_failure() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fixture = populate(dir.path(), None)?;
    let reference = fixture.regions.first().ok_or("region")?.reference()?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let key = chunk(&reference, 0)?;
    let bytes = store.meta.get(&key)?.ok_or("chunk")?;
    let changed =
        std::str::from_utf8(bytes.as_ref())?.replace("synthetic:backup", "synthetic:backuq");
    ensure(
        changed.as_bytes() != bytes.as_ref(),
        "fixture changed no bytes",
    )?;
    equal(changed.len(), bytes.len())?;
    inject(&store, &key, changed.as_bytes())?;
    assert_class(&store, CorruptionClass::DigestMismatch)
}

#[test]
fn unsupported_profile_and_region_protocols_are_incomplete_not_corrupt() -> TestResult {
    for family in ["profile", "region", "namespace"] {
        let dir = tempfile::tempdir()?;
        let fixture = populate(dir.path(), None)?;
        let store = Store::open_with(dir.path(), CACHE_BYTES)?;
        let key = match family {
            "profile" => crate::profiles::key(&fixture.profiles.first().ok_or("profile")?.id()?),
            "region" => descriptor(&fixture.regions.first().ok_or("region")?.reference()?),
            _ => "profiles:v2:future".to_string(),
        };
        inject(&store, key.as_bytes(), br#"{"version":2}"#)?;
        let before = Witness::capture(&store)?;
        let report = metadata_report(&store)?;
        equal(report.scope, Scope::Incomplete)?;
        ensure(
            report.corruptions.is_empty(),
            "unsupported version was mislabeled corruption",
        )?;
        ensure(
            !report.limits.is_empty(),
            "unsupported version lost its incomplete-scope reason",
        )?;
        ensure(
            !store.integrity()?.ok,
            "generic integrity certified unsupported registry metadata",
        )?;
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}

#[test]
fn entire_missing_registration_requires_explicit_expected_reference_oracle_after_restore(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source");
    let fixture = populate(&source, None)?;
    let profile = fixture.profiles.first().ok_or("profile")?;
    let reference = fixture.regions.first().ok_or("region")?.reference()?;
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        remove(&store, crate::profiles::key(&profile.id()?).as_bytes())?;
        remove(&store, descriptor(&reference).as_bytes())?;
        remove(&store, &chunk(&reference, 0)?)?;
        let report = metadata_report(&store)?;
        equal(report.scope, Scope::Complete)?;
        ensure(
            report.corruptions.is_empty(),
            "retained-only inventory invented evidence of a wholly missing registration",
        )?;
        ensure(
            store.integrity()?.ok,
            "fixture retained metadata should be structurally healthy",
        )?;
    }
    let backup = dir.path().join("backup");
    let restored = dir.path().join("restored");
    Store::backup(&source, &backup)?;
    Store::restore(&backup, &restored)?;
    let store = Store::open_with(&restored, CACHE_BYTES)?;
    let before = Witness::capture(&store)?;
    match store.profile(&profile.id()?) {
        Err(StoreError::ProfileRegistry(crate::ProfileRegistryError::MissingProfile { id })) => {
            equal(id, profile.id()?)?
        }
        other => return Err(format!("required missing profile={other:?}").into()),
    }
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            crate::RegionContractRegistryError::MissingContract { reference: missing },
        )) => equal(missing, reference)?,
        other => return Err(format!("required missing region={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn future_namespace_dispatch_precedes_v1_key_shape_and_size_assumptions() -> TestResult {
    for prefix in ["profiles:v2:", "region-contracts:v2/"] {
        let dir = tempfile::tempdir()?;
        let store = Store::open_with(dir.path(), CACHE_BYTES)?;
        let key = format!("{prefix}{}", "x".repeat(160));
        inject(&store, key.as_bytes(), br#"{"future":"shape"}"#)?;
        let before = Witness::capture(&store)?;
        let report = metadata_report(&store)?;
        equal(report.scope, Scope::Incomplete)?;
        ensure(
            report.corruptions.is_empty(),
            "future namespace was judged by the v1 key ceiling",
        )?;
        ensure(
            report
                .limits
                .iter()
                .any(|limit| limit.contains("unsupported registry namespace")),
            "future namespace dispatch was not explicit",
        )?;
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}
