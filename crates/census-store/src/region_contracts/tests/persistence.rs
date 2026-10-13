use super::*;

#[test]
fn registration_roundtrips_exact_payload_and_survives_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    {
        let store = Store::open_with(dir.path(), 8_388_608)?;
        store.append(
            Table::Schools,
            &serde_json::json!({"id":"synthetic-legacy", "note":"retained"}),
        )?;
        let before = Witness::capture(&store)?;
        equal(
            store.register_region_contract(&reference, &payload)?,
            RegionContractRegistration::Registered(reference.clone()),
        )?;
        equal(store.region_contract(&reference)?, payload.clone())?;
        let after = Witness::capture(&store)?;
        before.unchanged_evidence(&after)?;
        equal(after.sequence > before.sequence, true)?;
    }
    let reopened = Store::open_with(dir.path(), 8_388_608)?;
    equal(reopened.region_contract(&reference)?, payload)
}

#[test]
fn equivalent_reordered_and_duplicate_registration_preserves_every_retained_byte() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let one = assertion(1, "observed 2026-10-01")?;
    let two = assertion(2, "observed 2026-10-01")?;
    let first = RegionMembershipContractV1::new(input(7, vec![one.clone(), two.clone()])?)?;
    let second = RegionMembershipContractV1::new(input(7, vec![two.clone(), one, two])?)?;
    let reference = first.reference()?;
    equal(second.reference()?, reference.clone())?;
    equal(
        store.register_region_contract(&reference, &first)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let before = Witness::capture(&store)?;
    equal(
        store.register_region_contract(&reference, &second)?,
        RegionContractRegistration::Repeated(reference.clone()),
    )?;
    equal(store.region_contract(&reference)?, first)?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn different_contract_revision_retains_both_entries_without_replacing_history() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let first = contract(7)?;
    let second = contract(8)?;
    let first_ref = first.reference()?;
    let second_ref = second.reference()?;
    equal(
        store.register_region_contract(&first_ref, &first)?,
        RegionContractRegistration::Registered(first_ref.clone()),
    )?;
    let frozen = store.snapshot();
    equal(
        store.register_region_contract(&second_ref, &second)?,
        RegionContractRegistration::Registered(second_ref.clone()),
    )?;
    equal(store.region_contract(&first_ref)?, first.clone())?;
    equal(store.region_contract(&second_ref)?, second)?;
    equal(frozen.region_contract(&first_ref)?, first)
}

#[test]
fn registration_refuses_wrong_digest_and_revision_without_mutation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let computed = payload.reference()?;
    for reference in [
        ContractRefV1::new(1, 8, computed.digest())?,
        ContractRefV1::new(1, 7, &"0".repeat(64))?,
    ] {
        let before = Witness::capture(&store)?;
        match store.register_region_contract(&reference, &payload) {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::ImmutableConflict {
                    requested,
                    computed: observed,
                },
            )) => {
                equal(requested, reference)?;
                equal(observed, computed.clone())?;
            }
            other => return Err(format!("unexpected conflict outcome: {other:?}").into()),
        }
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}

#[test]
fn concurrent_equivalent_registration_commits_one_descriptor_and_payload() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let barrier = std::sync::Barrier::new(2);
    let (left, right) = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            barrier.wait();
            store.register_region_contract(&reference, &payload)
        });
        let right = scope.spawn(|| {
            barrier.wait();
            store.register_region_contract(&reference, &payload)
        });
        (left.join(), right.join())
    });
    let left = left.map_err(|_| "left registration panicked")??;
    let right = right.map_err(|_| "right registration panicked")??;
    let registered = RegionContractRegistration::Registered(reference.clone());
    let repeated = RegionContractRegistration::Repeated(reference.clone());
    equal(
        (left == registered && right == repeated) || (right == registered && left == repeated),
        true,
    )?;
    equal(store.region_contract(&reference)?, payload)?;
    let key = keys::descriptor(&reference)?;
    equal(store.meta.prefix(keys::chunk_prefix(&key)?).count(), 1)
}

#[test]
fn historical_run_and_uppercase_lineage_stay_byte_exact_during_registration() -> TestResult {
    use census_domain::model::{CensusRun, GradYear, RunManifest};
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let identity = StoreIdentity::parse(&"A".repeat(64))?;
    let lineage = serde_json::json!({"key":"store-identity", "at":"2020-01-02T03:04:05Z", "payload":identity});
    let legacy = RunManifest {
        store_identity: identity.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("year")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    let legacy_bytes = serde_json::to_vec(&legacy)?;
    let mut batch = store.db.batch();
    batch.insert(
        &store.journal,
        Store::journal_key("export-lineage-v1", "store-identity"),
        serde_json::to_vec(&lineage)?,
    );
    batch.insert(&store.meta, b"run-manifest", legacy_bytes.as_slice());
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let before = Witness::capture(&store)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    before.unchanged_evidence(&Witness::capture(&store)?)?;
    equal(
        store.selected_region_contract(&identity, &reference)?,
        payload,
    )?;
    equal(store.store_identity()?, identity.clone())?;
    equal(
        store.meta.get(b"run-manifest")?.map(|bytes| bytes.to_vec()),
        Some(legacy_bytes),
    )?;
    equal(store.run_manifest()?, Some(legacy))?;
    let lower = StoreIdentity::parse(&identity.as_str().to_ascii_lowercase())?;
    match store
        .snapshot()
        .selected_region_contract(&lower, &reference)
    {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::ForeignStore {
            expected,
            actual,
        })) => {
            equal(expected, lower)?;
            equal(actual, identity)
        }
        other => Err(format!("historical identity was lowercased: {other:?}").into()),
    }
}
