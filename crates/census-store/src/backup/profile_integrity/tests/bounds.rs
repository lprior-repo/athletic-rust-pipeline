use super::*;

#[test]
fn profile_inventory_charges_exact_key_and_value_bytes_before_decode() -> TestResult {
    let directory = tempfile::tempdir()?;
    let expected = populate(directory.path())?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let raw = store.db.snapshot();
    let key = crate::profiles::key(&expected.id()?);
    let size = raw
        .size_of(&store.meta, key.as_bytes())?
        .ok_or("profile size")?;
    let bytes = u64::try_from(key.len())?
        .checked_add(u64::from(size))
        .ok_or("encoded bytes")?;
    let before = Witness::capture(&store)?;
    let exact = check(&raw, &store.meta, 1, bytes)?;
    equal(exact.scope, Scope::Complete)?;
    equal(exact.records, 1)?;
    equal(exact.bytes, bytes)?;
    ensure(
        exact.corruptions.is_empty(),
        "exact budget rejected a valid profile",
    )?;
    for (records, ceiling) in [(0, bytes), (1, bytes.checked_sub(1).ok_or("bytes")?)] {
        let limited = check(&raw, &store.meta, records, ceiling)?;
        equal(limited.scope, Scope::Incomplete)?;
        ensure(
            limited.records <= records && limited.bytes <= ceiling,
            "overcharged budget",
        )?;
        ensure(
            limited.corruptions.is_empty(),
            "budget refusal called corrupt",
        )?;
        ensure(!limited.limits.is_empty(), "missing scope limit")?;
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn oversized_profile_refuses_store_snapshot_registration_integrity_and_backup() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("source");
    let expected = populate(&root)?;
    let length = crate::MAX_PROFILE_RECORD_BYTES
        .checked_add(1)
        .ok_or("length")?;
    {
        let store = Store::open_with(&root, CACHE_BYTES)?;
        let id = expected.id()?;
        inject(
            &store,
            crate::profiles::key(&id).as_bytes(),
            &vec![b'x'; length],
        )?;
        let before = Witness::capture(&store)?;
        for result in [
            store.profile(&id),
            store.snapshot().profile(&id),
            store.register_profile(&expected).map(|_| expected.clone()),
        ] {
            match result {
                Err(StoreError::ProfileRegistry(crate::ProfileRegistryError::Oversized {
                    bytes,
                    max,
                })) => {
                    equal(bytes, length)?;
                    equal(max, crate::MAX_PROFILE_RECORD_BYTES)?;
                }
                other => return Err(format!("oversized profile read={other:?}").into()),
            }
        }
        let report = metadata_report(&store)?;
        equal(report.corruptions.len(), 1)?;
        ensure(
            report.corruptions.iter().any(|finding| {
                finding.class == CorruptionClass::Oversized
                    && finding.detail.contains("value was not retrieved")
            }),
            "physical size must refuse before value retrieval",
        )?;
        ensure(
            !store.integrity()?.ok,
            "oversized profile falsely certified",
        )?;
        equal(Witness::capture(&store)?, before)?;
    }
    let destination = directory.path().join("backup");
    match Store::backup(&root, &destination) {
        Err(StoreError::Refused { detail }) => {
            ensure(detail.contains("profile metadata"), "wrong backup refusal")?
        }
        other => return Err(format!("oversized profile backup={other:?}").into()),
    }
    ensure(!destination.exists(), "refused backup was promoted")
}

#[test]
fn findings_cap_reports_incomplete_scope_and_preserves_retained_profile_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let count = MAX_FINDINGS.checked_add(1).ok_or("count")?;
    let mut batch = store.db.batch();
    for index in 0..count {
        batch.insert(
            &store.meta,
            format!("profiles:v1:{index:064x}").as_bytes(),
            b"{",
        );
    }
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let before = store.snapshot().sequence();
    let report = metadata_report(&store)?;
    equal(report.scope, Scope::Incomplete)?;
    equal(report.corruptions.len(), MAX_FINDINGS)?;
    ensure(
        report.limits.iter().any(|limit| limit.contains("findings")),
        "diagnostic suffix not disclosed",
    )?;
    equal(store.snapshot().sequence(), before)?;
    equal(
        store
            .meta
            .prefix(PREFIX)
            .take(count.checked_add(1).ok_or("bound")?)
            .count(),
        count,
    )
}
