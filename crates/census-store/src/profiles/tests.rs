use super::*;
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CensusRun, ContractRefV1, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1,
    ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, RunManifest, SchoolYear, Sport,
    StoreIdentity,
};
use fjall::{Keyspace, PersistMode, Readable, Snapshot};
use std::fmt::Debug;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    let contract = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Colorado,
        ],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse(as_of)?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: contract.clone(),
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-13")?,
        )?,
        completion_contract: contract.clone(),
        policies: ProfilePoliciesV1 {
            cohort: contract.clone(),
            event: contract.clone(),
            identity: contract.clone(),
            affiliation: contract.clone(),
            contact: contract.clone(),
            participation: contract.clone(),
            publication: contract,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

fn rows(snapshot: &Snapshot, keyspace: &Keyspace) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut held = Vec::new();
    held.try_reserve_exact(512)?;
    for guard in snapshot.prefix(keyspace, b"").take(513) {
        if held.len() == 512 {
            return Err("profile fixture inventory exceeds 512 rows".into());
        }
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        held.push((key.to_vec(), value.to_vec()));
    }
    Ok(held)
}

#[derive(Debug, PartialEq, Eq)]
struct Witness {
    sequence: u64,
    evidence: u64,
    derived: u64,
    source_digest: String,
    all_tables_digest: String,
    entities: Vec<(Vec<u8>, Vec<u8>)>,
    journal: Vec<(Vec<u8>, Vec<u8>)>,
    meta: Vec<(Vec<u8>, Vec<u8>)>,
    receipts: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Witness {
    fn capture(store: &Store) -> TestResult<Self> {
        let view = store.snapshot();
        let raw = store.db.snapshot();
        equal(view.sequence(), raw.seqno())?;
        Ok(Self {
            sequence: view.sequence(),
            evidence: view.evidence_generation(),
            derived: view.derived_generation(),
            source_digest: view.tables_digest(&[Table::Schools])?,
            all_tables_digest: view.tables_digest(&Table::ALL)?,
            entities: rows(&raw, &store.entities)?,
            journal: rows(&raw, &store.journal)?,
            meta: rows(&raw, &store.meta)?,
            receipts: rows(&raw, &store.receipts)?,
        })
    }

    fn same_evidence(&self, other: &Self) -> TestResult {
        equal(self.evidence, other.evidence)?;
        equal(self.derived, other.derived)?;
        equal(&self.source_digest, &other.source_digest)?;
        equal(&self.all_tables_digest, &other.all_tables_digest)?;
        equal(&self.entities, &other.entities)?;
        equal(&self.journal, &other.journal)?;
        equal(&self.receipts, &other.receipts)
    }
}

fn seed(store: &Store) -> TestResult {
    store.append(
        Table::Schools,
        &serde_json::json!({"id":"profile-fixture-school","note":"retained"}),
    )?;
    Ok(())
}

fn inject(store: &Store, id: &ProfileId, bytes: &[u8]) -> StoreResult<()> {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.insert(&store.meta, key(id).as_bytes(), bytes);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn envelope(profile: &QueryProfileV1) -> TestResult<serde_json::Value> {
    Ok(serde_json::to_value(Envelope {
        version: 1,
        id: &profile.id()?,
        payload: profile,
    })?)
}

#[test]
fn concurrent_independent_profiles_survive_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let first = profile("2026-10-11")?;
    let second = profile("2026-10-12")?;
    let first_id = first.id()?;
    let second_id = second.id()?;
    {
        let store = Store::open_with(dir.path(), 8_388_608)?;
        seed(&store)?;
        let barrier = std::sync::Barrier::new(2);
        let (left, right) = std::thread::scope(|scope| {
            let left = scope.spawn(|| {
                barrier.wait();
                store.register_profile(&first)
            });
            let right = scope.spawn(|| {
                barrier.wait();
                store.register_profile(&second)
            });
            (left.join(), right.join())
        });
        equal(
            left.map_err(|_| "left registration thread panicked")??,
            ProfileRegistration::Registered(first_id.clone()),
        )?;
        equal(
            right.map_err(|_| "right registration thread panicked")??,
            ProfileRegistration::Registered(second_id.clone()),
        )?;
        equal(store.profile(&first_id)?, first.clone())?;
        equal(store.profile(&second_id)?, second.clone())?;
        store.flush()?;
    }
    let reopened = Store::open_with(dir.path(), 8_388_608)?;
    equal(reopened.profile(&first_id)?, first)?;
    equal(reopened.profile(&second_id)?, second)
}

#[test]
fn equivalent_repeat_preserves_every_raw_byte_and_sequence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    seed(&store)?;
    let original = profile("2026-10-11")?;
    let before = Witness::capture(&store)?;
    equal(
        store.register_profile(&original)?,
        ProfileRegistration::Registered(original.id()?),
    )?;
    let registered = Witness::capture(&store)?;
    before.same_evidence(&registered)?;
    equal(
        registered.sequence,
        before.sequence.checked_add(1).ok_or("sequence")?,
    )?;
    let mut reordered = original.input().clone();
    reordered.jurisdictions.reverse();
    reordered.jurisdictions.push(UsJurisdiction::Michigan);
    reordered.graduation_years.push(GradYear::CO2027);
    let equivalent = QueryProfileV1::new(reordered)?;
    equal(
        store.register_profile(&equivalent)?,
        ProfileRegistration::Repeated(original.id()?),
    )?;
    equal(Witness::capture(&store)?, registered)?;
    Ok(())
}

#[test]
fn exact_missing_lookup_and_foreign_selection_have_no_writes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let profile = profile("2026-10-11")?;
    let before = Witness::capture(&store)?;
    match store.profile(&profile.id()?) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::MissingProfile { id })) => {
            equal(id, profile.id()?)?
        }
        other => return Err(format!("missing lookup={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)?;
    store.register_profile(&profile)?;
    let identity = store.store_identity()?;
    let foreign = StoreIdentity::parse(&"b".repeat(64))?;
    if foreign == identity {
        return Err("synthetic foreign identity equals generated identity".into());
    }
    let selection = ProfileSelectionV1::new(foreign, profile.id()?);
    let before = Witness::capture(&store)?;
    match store.selected_profile(&selection) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::Selection(
            ProfileSelectionError::ForeignStore,
        ))) => {}
        other => return Err(format!("foreign lookup={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)?;
    equal(store.profile(&profile.id()?)?, profile)
}

#[test]
fn pinned_reader_never_falls_back_to_later_live_registration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let owner = store.store_identity()?;
    let first = profile("2026-10-11")?;
    let second = profile("2026-10-12")?;
    store.register_profile(&first)?;
    let pinned = store.snapshot();
    store.register_profile(&second)?;
    equal(pinned.profile(&first.id()?)?, first.clone())?;
    let selection = ProfileSelectionV1::new(owner.clone(), first.id()?);
    equal(pinned.selected_profile(&selection)?, first)?;
    match pinned.profile(&second.id()?) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::MissingProfile { id })) => {
            equal(id, second.id()?)?
        }
        other => return Err(format!("pinned lookup={other:?}").into()),
    }
    let foreign = ProfileSelectionV1::new(StoreIdentity::parse(&"b".repeat(64))?, second.id()?);
    match pinned.selected_profile(&foreign) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::Selection(
            ProfileSelectionError::ForeignStore,
        ))) => {}
        other => return Err(format!("pinned foreign lookup={other:?}").into()),
    }
    equal(store.profile(&second.id()?)?, second)
}

#[test]
fn malformed_unknown_and_oversized_records_refuse_distinctly_without_writes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let profile = profile("2026-10-11")?;
    let id = profile.id()?;
    inject(&store, &id, b"{")?;
    let before = Witness::capture(&store)?;
    match store.profile(&id) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::Malformed {
            id: failed, ..
        })) => equal(failed, id.clone())?,
        other => return Err(format!("malformed={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)?;
    for (target, what) in [
        ("envelope", "profile envelope"),
        ("payload", "query profile payload"),
    ] {
        let mut raw = envelope(&profile)?;
        let object = if target == "envelope" {
            &mut raw
        } else {
            raw.get_mut("payload").ok_or("payload")?
        };
        object
            .as_object_mut()
            .ok_or("object")?
            .insert("version".to_string(), serde_json::json!(2));
        inject(&store, &id, &serde_json::to_vec(&raw)?)?;
        let before = Witness::capture(&store)?;
        match store.profile(&id) {
            Err(StoreError::ProfileRegistry(ProfileRegistryError::UnsupportedVersion {
                what: found_what,
                found,
            })) => {
                equal(found_what, what)?;
                equal(found, 2)?;
            }
            other => return Err(format!("unknown version={other:?}").into()),
        }
        equal(Witness::capture(&store)?, before)?;
    }
    let mut bytes = serde_json::to_vec(&envelope(&profile)?)?;
    bytes.resize(MAX_PROFILE_RECORD_BYTES, b' ');
    inject(&store, &id, &bytes)?;
    equal(store.profile(&id)?, profile.clone())?;
    bytes.push(b' ');
    inject(&store, &id, &bytes)?;
    let before = Witness::capture(&store)?;
    for result in [
        store.profile(&id).map(|_| ()),
        store.snapshot().profile(&id).map(|_| ()),
        store.register_profile(&profile).map(|_| ()),
    ] {
        match result {
            Err(StoreError::ProfileRegistry(ProfileRegistryError::Oversized { bytes, max })) => {
                equal(max, MAX_PROFILE_RECORD_BYTES)?;
                equal(bytes, max.checked_add(1).ok_or("ceiling")?)?;
            }
            other => return Err(format!("oversized={other:?}").into()),
        }
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn claimed_id_payload_mismatch_cannot_be_overwritten_by_registration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let first = profile("2026-10-11")?;
    let second = profile("2026-10-12")?;
    store.register_profile(&first)?;
    store.register_profile(&second)?;
    let mut spoof = envelope(&second)?;
    spoof
        .as_object_mut()
        .ok_or("object")?
        .insert("id".to_string(), serde_json::to_value(first.id()?)?);
    inject(&store, &first.id()?, &serde_json::to_vec(&spoof)?)?;
    let before = Witness::capture(&store)?;
    for result in [
        store.profile(&first.id()?),
        store.register_profile(&first).map(|_| first.clone()),
    ] {
        match result {
            Err(StoreError::ProfileRegistry(ProfileRegistryError::IdMismatch {
                requested,
                claimed,
                computed,
            })) => {
                equal(requested, first.id()?)?;
                equal(claimed, first.id()?)?;
                equal(computed, second.id()?)?;
            }
            other => return Err(format!("spoofed record={other:?}").into()),
        }
    }
    equal(store.profile(&second.id()?)?, second)?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn legacy_run_and_uppercase_lineage_survive_profile_registration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let owner = "A".repeat(64);
    let lineage_key = Store::journal_key("export-lineage-v1", "store-identity");
    let lineage = format!(
        "{{\"at\":\"2020-01-02T03:04:05Z\",\"key\":\"store-identity\",\"payload\":\"{owner}\"}}"
    );
    let legacy = RunManifest {
        store_identity: owner.clone(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    let mut batch = store.db.batch();
    batch.insert(&store.journal, lineage_key.as_slice(), lineage.as_bytes());
    batch.insert(&store.meta, b"run-manifest", serde_json::to_vec(&legacy)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let before = Witness::capture(&store)?;
    store.register_profile(&profile("2026-10-11")?)?;
    let after = Witness::capture(&store)?;
    before.same_evidence(&after)?;
    equal(store.run_manifest()?, Some(legacy.clone()))?;
    equal(store.store_identity()?.as_str(), owner.as_str())?;
    equal(
        store.meta.get(b"run-manifest")?.map(|bytes| bytes.to_vec()),
        before
            .meta
            .iter()
            .find(|(key, _)| key.as_slice() == b"run-manifest")
            .map(|(_, bytes)| bytes.clone()),
    )?;
    let mut foreign = legacy.clone();
    foreign.jurisdictions.push(UsJurisdiction::Ohio);
    match store.bind_run(&foreign) {
        Err(StoreError::Refused { .. }) => {}
        other => return Err(format!("legacy rebind={other:?}").into()),
    }
    equal(store.run_manifest()?, Some(legacy))?;
    equal(Witness::capture(&store)?, after)
}

#[test]
fn registering_configuration_never_binds_an_unbound_legacy_run_or_remints_lineage() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    seed(&store)?;
    let before = Witness::capture(&store)?;
    let profile = profile("2026-10-11")?;
    store.register_profile(&profile)?;
    let after = Witness::capture(&store)?;
    before.same_evidence(&after)?;
    equal(store.run_manifest()?, None)?;
    let selection = ProfileSelectionV1::new(StoreIdentity::parse(&"A".repeat(64))?, profile.id()?);
    match store.selected_profile(&selection) {
        Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => {}
        other => return Err(format!("unbound lineage selection={other:?}").into()),
    }
    equal(Witness::capture(&store)?, after)
}

#[test]
fn unknown_fields_and_invalid_semantic_payloads_refuse_without_overwrite() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let profile = profile("2026-10-11")?;
    let id = profile.id()?;
    for target in ["envelope", "payload"] {
        let mut raw = envelope(&profile)?;
        let object = if target == "envelope" {
            &mut raw
        } else {
            raw.get_mut("payload").ok_or("payload")?
        };
        object
            .as_object_mut()
            .ok_or("object")?
            .insert("extra".to_string(), serde_json::json!(1));
        inject(&store, &id, &serde_json::to_vec(&raw)?)?;
        let before = Witness::capture(&store)?;
        match store.profile(&id) {
            Err(StoreError::ProfileRegistry(ProfileRegistryError::Malformed { .. })) => {}
            other => return Err(format!("unknown field={other:?}").into()),
        }
        equal(Witness::capture(&store)?, before)?;
    }
    let mut raw = envelope(&profile)?;
    raw.get_mut("payload")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("payload")?
        .insert("sports".to_string(), serde_json::json!(["unknown"]));
    inject(&store, &id, &serde_json::to_vec(&raw)?)?;
    let before = Witness::capture(&store)?;
    match store.profile(&id) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::InvalidPayload(
            ProfileError::UnknownSport,
        ))) => {}
        other => return Err(format!("invalid sport={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn equivalent_profiles_in_distinct_stores_have_one_semantic_id_and_foreign_selection_refusal(
) -> TestResult {
    let left_dir = tempfile::tempdir()?;
    let right_dir = tempfile::tempdir()?;
    let left = Store::open_with(left_dir.path(), 8_388_608)?;
    let right = Store::open_with(right_dir.path(), 8_388_608)?;
    let profile = profile("2026-10-11")?;
    equal(
        left.register_profile(&profile)?,
        ProfileRegistration::Registered(profile.id()?),
    )?;
    equal(
        right.register_profile(&profile)?,
        ProfileRegistration::Registered(profile.id()?),
    )?;
    let left_owner = left.store_identity()?;
    let right_owner = right.store_identity()?;
    if left_owner == right_owner {
        return Err("independent installations share an unexpected identity".into());
    }
    let selection = ProfileSelectionV1::new(left_owner, profile.id()?);
    equal(left.selected_profile(&selection)?, profile.clone())?;
    let before = Witness::capture(&right)?;
    match right.selected_profile(&selection) {
        Err(StoreError::ProfileRegistry(ProfileRegistryError::Selection(
            ProfileSelectionError::ForeignStore,
        ))) => {}
        other => return Err(format!("foreign equivalent selection={other:?}").into()),
    }
    equal(Witness::capture(&right)?, before)?;
    equal(right.profile(&profile.id()?)?, profile)
}

#[test]
fn future_envelope_dispatch_does_not_assume_a_v1_payload_shape() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let id = profile("2026-10-11")?.id()?;
    for raw in [
        br#"{"version":2}"#.as_slice(),
        br#"{"version":2,"payload":"different-future-schema"}"#.as_slice(),
        br#"{"version":1,"payload":{"version":2}}"#.as_slice(),
    ] {
        inject(&store, &id, raw)?;
        let before = Witness::capture(&store)?;
        match store.profile(&id) {
            Err(StoreError::ProfileRegistry(ProfileRegistryError::UnsupportedVersion {
                found,
                ..
            })) => equal(found, 2)?,
            other => return Err(format!("future format was decoded as v1: {other:?}").into()),
        }
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}
