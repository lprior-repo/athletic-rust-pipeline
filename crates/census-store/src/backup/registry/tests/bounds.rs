use super::*;

#[test]
fn metadata_window_counts_both_families_and_charges_exact_key_plus_value_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    populate(dir.path(), None)?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let raw = store.db.snapshot();
    let mut records = 0_u64;
    let mut bytes = 0_u64;
    for prefix in PREFIXES {
        for guard in raw.prefix(&store.meta, prefix).take(32) {
            let (key, value) = guard.into_inner()?;
            records = records.checked_add(1).ok_or("records")?;
            bytes = bytes
                .checked_add(u64::try_from(key.len())?)
                .and_then(|sum| sum.checked_add(u64::try_from(value.len()).ok()?))
                .ok_or("bytes")?;
        }
    }
    ensure(
        records > 4,
        "fixture must include profiles, descriptors and chunks",
    )?;
    let before = Witness::capture(&store)?;
    let exact = check(&raw, &store.meta, records, bytes)?;
    equal(exact.scope, Scope::Complete)?;
    equal(exact.records, records)?;
    equal(exact.bytes, bytes)?;
    ensure(exact.corruptions.is_empty(), "exact admitted scope failed")?;
    for (rows, size) in [
        (records.checked_sub(1).ok_or("rows")?, bytes),
        (records, bytes.checked_sub(1).ok_or("bytes")?),
    ] {
        let closed = check(&raw, &store.meta, rows, size)?;
        equal(closed.scope, Scope::Incomplete)?;
        ensure(
            closed.records <= rows && closed.bytes <= size,
            "metadata budget was overcharged",
        )?;
        ensure(
            closed.corruptions.is_empty(),
            "admission refusal was mislabeled corruption",
        )?;
        ensure(
            closed
                .limits
                .iter()
                .any(|limit| limit.contains("key-plus-value bytes")),
            "metadata scope limit is not explicit",
        )?;
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn oversized_physical_profile_is_reported_before_retrieval_or_decode() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fixture = populate(dir.path(), None)?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let key = crate::profiles::key(&fixture.profiles.first().ok_or("profile")?.id()?);
    let length = crate::MAX_PROFILE_RECORD_BYTES
        .checked_add(1)
        .ok_or("length")?;
    inject(&store, key.as_bytes(), &vec![b'x'; length])?;
    let before = Witness::capture(&store)?;
    let report = check(&store.db.snapshot(), &store.meta, MAX_RECORDS, MAX_BYTES)?;
    ensure(
        report.corruptions.iter().any(|finding| {
            finding.class == CorruptionClass::Oversized
                && finding.detail.contains("value was not retrieved")
        }),
        "oversize was not classified at preflight",
    )?;
    ensure(
        !store.integrity()?.ok,
        "oversized metadata omitted from generic integrity",
    )?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn diagnostic_cap_reports_incomplete_scope_and_preserves_every_retained_record() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let mut batch = store.db.batch();
    for index in 0..MAX_FINDINGS.checked_add(1).ok_or("findings")? {
        batch.insert(
            &store.meta,
            format!("profiles:v1:{index:064x}").as_bytes(),
            b"{",
        );
    }
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let before = store.snapshot().sequence();
    let report = check(&store.db.snapshot(), &store.meta, MAX_RECORDS, MAX_BYTES)?;
    equal(report.scope, Scope::Incomplete)?;
    equal(report.corruptions.len(), MAX_FINDINGS)?;
    ensure(
        report.limits.iter().any(|limit| limit.contains("findings")),
        "capped diagnostics did not disclose incomplete verification",
    )?;
    equal(store.snapshot().sequence(), before)?;
    equal(
        store
            .meta
            .prefix(b"profiles:")
            .take(MAX_FINDINGS.checked_add(2).ok_or("bound")?)
            .count(),
        MAX_FINDINGS.checked_add(1).ok_or("count")?,
    )
}

#[test]
fn pinned_metadata_check_ignores_later_live_corruption_and_never_repairs_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fixture = populate(dir.path(), None)?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let pinned = store.db.snapshot();
    let key = crate::profiles::key(&fixture.profiles.first().ok_or("profile")?.id()?);
    inject(&store, key.as_bytes(), b"{")?;
    let before = Witness::capture(&store)?;
    let old = check(&pinned, &store.meta, MAX_RECORDS, MAX_BYTES)?;
    equal(old.scope, Scope::Complete)?;
    ensure(
        old.corruptions.is_empty(),
        "pinned checker read later live metadata",
    )?;
    let current = check(&store.db.snapshot(), &store.meta, MAX_RECORDS, MAX_BYTES)?;
    ensure(
        current
            .corruptions
            .iter()
            .any(|finding| finding.class == CorruptionClass::Undecodable),
        "fresh checker omitted retained corruption",
    )?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn complete_integrity_keeps_registry_budget_fixed_and_returns_incomplete_without_mutation(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    let base = profile("2026-10-13", region(1, 0)?.reference()?)?;
    for group in 0..16_u32 {
        let mut batch = store.db.batch();
        for offset in 0..64_u32 {
            let revision = group
                .checked_mul(64)
                .and_then(|value| value.checked_add(offset))
                .and_then(|value| value.checked_add(1))
                .ok_or("revision")?;
            let mut input = base.input().clone();
            input.source_plan = ContractRefV1::new(1, revision, &"a".repeat(64))?;
            let profile = QueryProfileV1::new(input)?;
            let id = profile.id()?;
            let mut raw =
                serde_json::to_vec(&serde_json::json!({"version":1,"id":&id,"payload":&profile}))?;
            ensure(
                raw.len() < crate::MAX_PROFILE_RECORD_BYTES,
                "fixture profile exceeds envelope",
            )?;
            raw.resize(crate::MAX_PROFILE_RECORD_BYTES, b' ');
            batch.insert(&store.meta, crate::profiles::key(&id), raw);
        }
        batch.durability(Some(PersistMode::SyncData)).commit()?;
    }
    let sequence = store.snapshot().sequence();
    let report = store.integrity_complete()?;
    ensure(
        !report.ok,
        "complete integrity silently lifted the fixed metadata budget",
    )?;
    ensure(
        report.corruptions.is_empty(),
        "budget exhaustion was mislabeled metadata corruption",
    )?;
    ensure(
        report.decode_limits.iter().any(|limit| {
            limit.contains("registry integrity stopped") && limit.contains("key-plus-value bytes")
        }),
        "complete integrity omitted actionable metadata continuation limitation",
    )?;
    equal(store.snapshot().sequence(), sequence)?;
    let view = store.db.snapshot();
    let mut count = 0_u64;
    for guard in view.prefix(&store.meta, b"profiles:").take(1025) {
        let key = guard.key()?;
        equal(
            view.size_of(&store.meta, &key)?,
            Some(u32::try_from(crate::MAX_PROFILE_RECORD_BYTES)?),
        )?;
        count = count.checked_add(1).ok_or("record count")?;
    }
    equal(count, 1024)
}
