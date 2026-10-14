use super::*;

#[test]
fn registration_adds_only_configuration_to_nonempty_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    seed(&store)?;
    let contract = contract(7)?;
    let reference = contract.reference()?;
    let before = Witness::capture(&store)?;
    equal(
        store.register_region_contract(&contract)?,
        RegionRegistration::Registered(reference.clone()),
    )?;
    let after = Witness::capture(&store)?;
    before.same_evidence(&after)?;
    equal(
        after.sequence,
        before.sequence.checked_add(1).ok_or("sequence overflow")?,
    )?;
    let mut expected_meta = before.meta;
    expected_meta.push((
        key(&reference).into_bytes(),
        census_domain::model::serialized_bytes(&Envelope {
            version: 1,
            reference: &reference,
            payload: &contract,
        })?,
    ));
    expected_meta.sort_unstable();
    equal(after.meta, expected_meta)?;
    equal(store.region_contract(&reference)?, contract)
}

#[test]
fn exact_and_normalized_repeats_preserve_every_byte_and_sequence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    seed(&store)?;
    let original = contract(7)?;
    let reference = original.reference()?;
    equal(
        store.register_region_contract(&original)?,
        RegionRegistration::Registered(reference.clone()),
    )?;
    let before = Witness::capture(&store)?;
    equal(
        store.register_region_contract(&original)?,
        RegionRegistration::Repeated(reference.clone()),
    )?;
    equal(&Witness::capture(&store)?, &before)?;
    let mut reordered = original.input().clone();
    reordered.assertions.reverse();
    reordered
        .assertions
        .extend(original.input().assertions.clone());
    reordered
        .jurisdictions
        .extend(original.input().jurisdictions.clone());
    let equivalent = RegionMembershipContractV1::new(reordered)?;
    equal(
        store.register_region_contract(&equivalent)?,
        RegionRegistration::Repeated(reference.clone()),
    )?;
    equal(&Witness::capture(&store)?, &before)?;
    equal(store.region_contract(&reference)?, original)
}

#[test]
fn pinned_snapshots_never_read_later_registrations_or_corruption() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let first = contract(7)?;
    let second = contract(8)?;
    let first_ref = first.reference()?;
    let second_ref = second.reference()?;
    let empty = store.snapshot();
    store.register_region_contract(&first)?;
    let pinned = store.snapshot();
    store.register_region_contract(&second)?;
    match empty.region_contract(&first_ref) {
        Err(RegionRegistryError::Missing { reference }) => equal(reference, first_ref.clone())?,
        other => return Err(format!("pre-registration snapshot={other:?}").into()),
    }
    equal(pinned.region_contract(&first_ref)?, first.clone())?;
    match pinned.region_contract(&second_ref) {
        Err(RegionRegistryError::Missing { reference }) => equal(reference, second_ref.clone())?,
        other => return Err(format!("pinned later registration={other:?}").into()),
    }
    equal(store.region_contract(&second_ref)?, second)?;
    inject(&store, &first_ref, b"{")?;
    match store.region_contract(&first_ref) {
        Err(RegionRegistryError::Malformed(_)) => {}
        other => return Err(format!("live corrupted record={other:?}").into()),
    }
    equal(pinned.region_contract(&first_ref)?, first)
}

#[test]
fn registered_revisions_survive_close_and_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let first = contract(7)?;
    let second = contract(8)?;
    let before = {
        let store = Store::open_with(directory.path(), CACHE_BYTES)?;
        seed(&store)?;
        store.register_region_contract(&first)?;
        store.register_region_contract(&second)?;
        store.flush()?;
        Witness::capture(&store)?
    };
    let reopened = Store::open_with(directory.path(), CACHE_BYTES)?;
    equal(
        reopened.region_contract(&first.reference()?)?,
        first.clone(),
    )?;
    equal(reopened.region_contract(&second.reference()?)?, second)?;
    equal(
        reopened.register_region_contract(&first)?,
        RegionRegistration::Repeated(first.reference()?),
    )?;
    equal(Witness::capture(&reopened)?, before)
}

#[test]
fn cold_backup_restore_preserves_new_configuration_and_existing_data() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let backup = directory.path().join("backup");
    let restored = directory.path().join("restored");
    let first = contract(7)?;
    let second = contract(8)?;
    let before = {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        seed(&store)?;
        store.register_region_contract(&first)?;
        let existing = Witness::capture(&store)?;
        store.register_region_contract(&second)?;
        let configured = Witness::capture(&store)?;
        existing.same_evidence(&configured)?;
        store.flush()?;
        configured
    };
    Store::backup(&source, &backup)?;
    Store::restore(&backup, &restored)?;
    let store = Store::open_with(&restored, CACHE_BYTES)?;
    equal(store.region_contract(&first.reference()?)?, first)?;
    equal(store.region_contract(&second.reference()?)?, second.clone())?;
    equal(
        store.register_region_contract(&second)?,
        RegionRegistration::Repeated(second.reference()?),
    )?;
    equal(Witness::capture(&store)?, before)
}
