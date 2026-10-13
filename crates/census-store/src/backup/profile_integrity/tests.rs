use super::*;
use crate::{ProfileRegistration, Store};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use fjall::{PersistMode, Readable};
use std::fmt::Debug;
use std::path::Path;

mod bounds;
mod corruption;
mod preservation;

const CACHE_BYTES: u64 = 8_388_608;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn ensure(condition: bool, detail: &str) -> TestResult {
    if !condition {
        return Err(detail.to_string().into());
    }
    Ok(())
}

fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    let contract = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::CrossCountry],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse(as_of)?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("season")?,
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

fn populate(root: &Path) -> TestResult<QueryProfileV1> {
    let store = Store::open_with(root, CACHE_BYTES)?;
    let profile = profile("2026-10-11")?;
    equal(
        store.register_profile(&profile)?,
        ProfileRegistration::Registered(profile.id()?),
    )?;
    store.flush()?;
    Ok(profile)
}

fn inject(store: &Store, key: &[u8], bytes: &[u8]) -> TestResult {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.insert(&store.meta, key, bytes);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn metadata_report(store: &Store) -> TestResult<Report> {
    Ok(check(
        &store.db.snapshot(),
        &store.meta,
        MAX_RECORDS,
        MAX_BYTES,
    )?)
}

fn raw_rows(snapshot: &Snapshot, keyspace: &Keyspace) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    let mut bytes = 0_usize;
    for guard in snapshot.prefix(keyspace, b"").take(257) {
        ensure(rows.len() < 256, "fixture row ceiling")?;
        let (key, value) = guard.into_inner()?;
        bytes = bytes
            .checked_add(key.len())
            .and_then(|count| count.checked_add(value.len()))
            .ok_or("fixture bytes")?;
        ensure(bytes <= 4 * 1024 * 1024, "fixture byte ceiling")?;
        rows.push((key.to_vec(), value.to_vec()));
    }
    Ok(rows)
}

#[derive(Debug, PartialEq, Eq)]
struct Witness {
    sequence: u64,
    evidence: u64,
    derived: u64,
    table_digest: String,
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
            table_digest: view.tables_digest(&crate::Table::ALL)?,
            entities: raw_rows(&raw, &store.entities)?,
            journal: raw_rows(&raw, &store.journal)?,
            meta: raw_rows(&raw, &store.meta)?,
            receipts: raw_rows(&raw, &store.receipts)?,
        })
    }
}

fn assert_class(store: &Store, class: CorruptionClass) -> TestResult {
    let before = Witness::capture(store)?;
    let report = metadata_report(store)?;
    ensure(
        report
            .corruptions
            .iter()
            .any(|finding| finding.class == class),
        "expected profile corruption absent",
    )?;
    ensure(
        !store.integrity()?.ok,
        "integrity omitted retained profiles",
    )?;
    equal(Witness::capture(store)?, before)
}
