use super::*;

#[test]
fn token_remains_unbound_when_first_binding_intervenes_after_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = store.store_identity()?;
    let snapshot = store.snapshot();
    let lineage = super::super::lineage(&store, &snapshot)?;
    let coordinates = (
        snapshot.sequence(),
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
    );
    let bound = manifest(identity.as_str())?;
    store.bind_run(&bound)?;
    let token = FrozenInputToken::capture(&snapshot, &lineage)?;
    check!(eq; lineage.run, None);
    check!(eq; lineage.cohort, None);
    check!(eq; token.run_manifest_digest, None);
    check!(eq; token.snapshot_sequence, coordinates.0);
    check!(eq; snapshot.run_manifest()?, None);
    check!(eq; (snapshot.sequence(), snapshot.evidence_generation(), snapshot.derived_generation()), coordinates);
    let fresh = store.snapshot();
    let current = super::super::lineage(&store, &fresh)?;
    let current_token = FrozenInputToken::capture(&fresh, &current)?;
    check!(eq; current.run, Some(bound.run));
    check!(eq; current.cohort, Some(bound.cohort));
    check!(eq; current_token.run_manifest_digest, Some(serialized_digest(&bound)?));
    check!(eq; current.source_digest, lineage.source_digest);
    Ok(())
}

#[test]
fn token_refuses_prebinding_lineage_with_a_fresh_bound_snapshot() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = store.store_identity()?;
    let old = store.snapshot();
    let lineage = super::super::lineage(&store, &old)?;
    store.bind_run(&manifest(identity.as_str())?)?;
    let current = store.snapshot();
    invariant(FrozenInputToken::capture(&current, &lineage))
}

#[test]
fn token_refuses_sequence_identity_run_and_cohort_mismatches() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = store.store_identity()?;
    let bound = manifest(identity.as_str())?;
    store.bind_run(&bound)?;
    let snapshot = store.snapshot();
    let lineage = super::super::lineage(&store, &snapshot)?;
    let mut sequence = lineage.clone();
    sequence.snapshot_sequence = sequence
        .snapshot_sequence
        .checked_add(1)
        .ok_or("sequence overflow")?;
    invariant(FrozenInputToken::capture(&snapshot, &sequence))?;
    let mut identity = lineage.clone();
    identity.store_identity = if lineage.store_identity == "a".repeat(64) {
        "b".repeat(64)
    } else {
        "a".repeat(64)
    };
    invariant(FrozenInputToken::capture(&snapshot, &identity))?;
    let mut run = lineage.clone();
    run.run = Some(CensusRun::new(bound.run.season(), 8).ok_or("fixture alternate run")?);
    invariant(FrozenInputToken::capture(&snapshot, &run))?;
    let mut cohort = lineage.clone();
    cohort.cohort = Some(GradYear::new(2028).ok_or("fixture alternate cohort")?);
    invariant(FrozenInputToken::capture(&snapshot, &cohort))?;
    let mut absent_run = lineage.clone();
    absent_run.run = None;
    invariant(FrozenInputToken::capture(&snapshot, &absent_run))?;
    let mut absent_cohort = lineage;
    absent_cohort.cohort = None;
    invariant(FrozenInputToken::capture(&snapshot, &absent_cohort))
}

#[test]
fn token_refuses_fabricated_binding_in_an_unbound_snapshot() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = store.store_identity()?;
    let snapshot = store.snapshot();
    let mut lineage = super::super::lineage(&store, &snapshot)?;
    let bound = manifest(identity.as_str())?;
    lineage.run = Some(bound.run);
    lineage.cohort = Some(bound.cohort);
    invariant(FrozenInputToken::capture(&snapshot, &lineage))
}

#[test]
fn token_refuses_a_manifest_whose_identity_differs_from_the_pinned_identity() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.store_identity()?;
    let mut bound = manifest(&"c".repeat(64))?;
    let original = store.store_identity()?;
    if bound.store_identity == original.as_str() {
        bound.store_identity = "d".repeat(64);
    }
    let before = store.snapshot();
    let mut lineage = super::super::lineage(&store, &before)?;
    lineage.run = Some(bound.run);
    lineage.cohort = Some(bound.cohort);
    store.bind_run(&bound)?;
    let current = store.snapshot();
    lineage.snapshot_sequence = current.sequence();
    invariant(FrozenInputToken::capture(&current, &lineage))
}

#[test]
fn bound_token_preserves_the_complete_manifest_bytes_digest_and_jurisdiction_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = "0123456789abcdef".repeat(4);
    store.journal_done("export-lineage-v1", "store-identity", &identity)?;
    let bound = manifest(&identity)?;
    store.bind_run(&bound)?;
    let snapshot = store.snapshot();
    let mut lineage = super::super::lineage(&store, &snapshot)?;
    lineage.generated_on = "2026-10-13".into();
    let token = FrozenInputToken::capture(&snapshot, &lineage)?;
    let golden = br#"{"store_identity":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","run":{"season":2026,"revision":7},"cohort":2027,"jurisdictions":["WI","IL","WI"]}"#;
    check!(eq; serialized_bytes(&bound)?, golden.to_vec());
    check!(eq; serialized_digest(&bound)?, "d7aa5a60ee50e546ac1cb9165331d39eb1fb030cac94e8ddba895e95f5bbfef4");
    check!(eq; snapshot.run_manifest()?, Some(bound.clone()));
    check!(eq; token.run_manifest_digest, Some(serialized_digest(&bound)?));
    let encoded = serialized_bytes(&token)?;
    let golden_token = format!(
        "{{\"protocol\":3,\"store_identity\":\"{identity}\",\"run_manifest_digest\":\"{}\",\"store_schema\":2,\"key_format\":1,\"logical_schema\":3,\"semantic_policy\":18,\"input_root\":\"{}\",\"snapshot_sequence\":{},\"as_of\":\"2026-10-13\"}}",
        serialized_digest(&bound)?, lineage.source_digest, snapshot.sequence()
    );
    check!(eq; encoded, golden_token.as_bytes());
    check!(eq; token.digest()?, super::super::codec::sha256_hex(golden_token.as_bytes()));
    let mut historical = token.clone();
    historical.semantic_policy = 17;
    let historical_golden =
        golden_token.replace("\"semantic_policy\":18,", "\"semantic_policy\":17,");
    check!(eq; serialized_bytes(&historical)?, historical_golden.as_bytes());
    check!(eq; historical.digest()?, super::super::codec::sha256_hex(historical_golden.as_bytes()));
    check!(ne; token.digest()?, historical.digest()?);
    let mut reordered = bound.clone();
    reordered.jurisdictions.swap(0, 1);
    check!(ne; serialized_digest(&reordered)?, serialized_digest(&bound)?);
    reordered.jurisdictions = vec![UsJurisdiction::Illinois, UsJurisdiction::Wisconsin];
    check!(ne; serialized_digest(&reordered)?, serialized_digest(&bound)?);
    Ok(())
}
