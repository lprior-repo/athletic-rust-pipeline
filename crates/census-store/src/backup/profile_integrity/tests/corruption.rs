use super::*;

#[test]
fn malformed_retained_profile_is_corruption_without_entity_or_registry_mutation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let expected = populate(directory.path())?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    inject(
        &store,
        crate::profiles::key(&expected.id()?).as_bytes(),
        b"{",
    )?;
    assert_class(&store, CorruptionClass::Undecodable)
}

#[test]
fn changed_valid_payload_under_old_profile_id_is_digest_mismatch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let expected = populate(directory.path())?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let changed = profile("2026-10-12")?;
    let envelope = serde_json::json!({"version": 1, "id": expected.id()?, "payload": changed});
    inject(
        &store,
        crate::profiles::key(&expected.id()?).as_bytes(),
        &serde_json::to_vec(&envelope)?,
    )?;
    assert_class(&store, CorruptionClass::DigestMismatch)
}

#[test]
fn noncanonical_profile_key_is_undecodable() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    inject(&store, b"profiles:v1:not-a-digest", b"{}")?;
    assert_class(&store, CorruptionClass::Undecodable)
}

#[test]
fn unsupported_profile_protocols_are_incomplete_and_never_corruption() -> TestResult {
    let directory = tempfile::tempdir()?;
    let expected = populate(directory.path())?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    for (key, bytes) in [
        (
            crate::profiles::key(&expected.id()?),
            br#"{"version":2}"#.as_slice(),
        ),
        (
            format!("profiles:v2:{}", "x".repeat(160)),
            br#"{"future":"shape"}"#.as_slice(),
        ),
    ] {
        inject(&store, key.as_bytes(), bytes)?;
        let before = Witness::capture(&store)?;
        let report = metadata_report(&store)?;
        equal(report.scope, Scope::Incomplete)?;
        ensure(
            report.corruptions.is_empty(),
            "future format called corrupt",
        )?;
        ensure(!report.limits.is_empty(), "unsupported reason absent")?;
        ensure(!store.integrity()?.ok, "future profile falsely certified")?;
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}

#[test]
fn pinned_profile_check_never_reads_later_live_corruption() -> TestResult {
    let directory = tempfile::tempdir()?;
    let expected = populate(directory.path())?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let pinned = store.db.snapshot();
    inject(
        &store,
        crate::profiles::key(&expected.id()?).as_bytes(),
        b"{",
    )?;
    let before = Witness::capture(&store)?;
    let historical = check(&pinned, &store.meta, MAX_RECORDS, MAX_BYTES)?;
    equal(historical.scope, Scope::Complete)?;
    ensure(
        historical.corruptions.is_empty(),
        "pinned check saw a later write",
    )?;
    assert_class(&store, CorruptionClass::Undecodable)?;
    equal(Witness::capture(&store)?, before)
}
