use super::*;

#[test]
fn missing_present_and_surplus_chunks_have_distinct_completion_outcomes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/a", &manifest_value("row/a", 1, 4)?)?;
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "logical root has missing physical chunks",
    )?;
    let key = chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?;
    store
        .journal
        .insert(key, chunk_value("row/a", 0, 1, b"null")?)?;
    check!(eq; roots(&store.snapshot())?.logical_roots(), 1);
    let surplus = chunk_key(&Store::journal_key(PHASE, "row/a"), 1)?;
    store.journal.insert(surplus, b"extra".as_slice())?;
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "chunk is orphaned, missing, or surplus",
    )?;
    Ok(())
}

#[test]
fn orphan_chunk_cannot_be_promoted_to_a_logical_root() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let key = chunk_key(&Store::journal_key(PHASE, "row/orphan"), 0)?;
    store.journal.insert(key, b"opaque".as_slice())?;
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "chunk has no preceding logical root",
    )?;
    let fingerprint = store
        .snapshot()
        .journal_namespace_fingerprint(namespace()?, budget(1))?;
    check!(eq; fingerprint.physical_rows(), 1);
    Ok(())
}

#[test]
fn all_nearby_binary_chunk_suffix_lengths_require_exactly_four_ordinal_bytes() -> TestResult {
    for ordinal_bytes in 0..=8 {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let mut key = Store::journal_key(PHASE, "row/orphan");
        key.push(1);
        let length = key
            .len()
            .checked_add(ordinal_bytes)
            .ok_or(StoreError::CounterOverflow)?;
        key.resize(length, 0);
        store.journal.insert(key, b"opaque".as_slice())?;
        let reason = if ordinal_bytes == 4 {
            "chunk has no preceding logical root"
        } else {
            "chunk key has an unsupported suffix"
        };
        descriptor(roots(&store.snapshot()), "row/", reason)?;
    }
    Ok(())
}

#[test]
fn middle_ordinal_gap_refuses_instead_of_counting_two_chunks_as_complete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/a", &manifest_value("row/a", 3, 1_000_001)?)?;
    for ordinal in [0, 2] {
        store.journal.insert(
            chunk_key(&Store::journal_key(PHASE, "row/a"), ordinal)?,
            b"opaque".as_slice(),
        )?;
    }
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "chunk is orphaned, missing, or surplus",
    )?;
    Ok(())
}

#[test]
fn missing_final_chunk_refuses_before_the_following_root_callback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/a", &manifest_value("row/a", 2, 500_001)?)?;
    store.journal.insert(
        chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?,
        b"opaque".as_slice(),
    )?;
    put(&store, PHASE, "row/b", 1)?;
    let mut seen = Vec::new();
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, budget(3), |key| {
            seen.push(key.to_string());
            Ok(())
        });
    descriptor(refused, "row/", "logical root has missing physical chunks")?;
    check!(eq; seen, vec!["row/a".to_string()]);
    Ok(())
}

#[test]
fn inline_replacement_with_retained_chunks_refuses_canonical_inventory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/a", &manifest_value("row/a", 1, 4)?)?;
    store.journal.insert(
        chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?,
        b"opaque".as_slice(),
    )?;
    put(&store, PHASE, "row/a", 7)?;
    check!(eq; store.snapshot().journal_payload(PHASE, "row/a")?, Some(json!({"revision": 7})));
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "chunk is orphaned, missing, or surplus",
    )?;
    Ok(())
}

#[test]
fn ready_alias_with_stale_binary_chunks_refuses_canonical_inventory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/a", &ready_alias()?)?;
    store.journal.insert(
        chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?,
        b"opaque".as_slice(),
    )?;
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "chunk is orphaned, missing, or surplus",
    )?;
    Ok(())
}

#[test]
fn bounded_ordinal_cases_require_zero_start_contiguity_and_exact_count() -> TestResult {
    let cases: &[(&[usize], Option<&str>)] = &[
        (&[], Some("logical root has missing physical chunks")),
        (&[0], Some("logical root has missing physical chunks")),
        (&[1], Some("chunk is orphaned, missing, or surplus")),
        (&[0, 2], Some("chunk is orphaned, missing, or surplus")),
        (&[0, 1, 2], None),
        (
            &[0, 1, 2, 3],
            Some("chunk is orphaned, missing, or surplus"),
        ),
        (
            &[4_294_967_295],
            Some("chunk is orphaned, missing, or surplus"),
        ),
    ];
    for (ordinals, refusal) in cases {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        raw(&store, "row/a", &manifest_value("row/a", 3, 1_000_001)?)?;
        for ordinal in *ordinals {
            store.journal.insert(
                chunk_key(&Store::journal_key(PHASE, "row/a"), *ordinal)?,
                b"opaque".as_slice(),
            )?;
        }
        let result = roots(&store.snapshot());
        match refusal {
            Some(reason) => descriptor(result, "row/", reason)?,
            None => {
                let inventory = result?;
                check!(eq; inventory.logical_roots(), 1);
                check!(eq; inventory.fingerprint().physical_rows(), 4);
            }
        }
    }
    Ok(())
}
