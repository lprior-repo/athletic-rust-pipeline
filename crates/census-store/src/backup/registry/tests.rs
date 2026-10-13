use super::*;
use crate::{ProfileRegistration, RegionContractRegistration, Store, Table};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use fjall::{PersistMode, Readable};
use std::fmt::Debug;
use std::path::Path;

mod bounds;
mod corruption;
mod preservation;
mod roundtrip;

const CACHE_BYTES: u64 = 8_388_608;
const FIXTURE_ROWS: usize = 256;
const FIXTURE_BYTES: usize = 4 * 1024 * 1024;
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

fn region(revision: u32, count: usize) -> TestResult<RegionMembershipContractV1> {
    let range = ProfileDateRangeV1::new(
        ProfileDate::parse("2026-01-01")?,
        ProfileDate::parse("2027-12-31")?,
    )?;
    let mut assertions = Vec::new();
    for index in 0..count {
        let school = RegionSchoolV1::new(
            Id::mint("sch", &["backup-fixture", &index.to_string()]),
            UsJurisdiction::Michigan,
        )?;
        let mut evidence = Evidence::parsed(
            SourceRef::new(
                "synthetic-backup",
                Some("https://example.invalid/registry".to_string()),
            ),
            "2026-10-01T12:00:00Z",
        );
        evidence.note = Some("x".repeat(2048));
        assertions.push(RegionAssertionV1::new(RegionAssertionInputV1 {
            subject: RegionSubjectV1::School(school),
            context: RegionContextV1 {
                sport: Sport::CrossCountry,
                category: Gender::Boys,
                school_year: SchoolYear::new(2026).ok_or("season")?,
            },
            effective: RegionEffectiveV1::Inclusive(range.clone()),
            statement: RegionStatementV1::Member,
            provenance: RegionEvidenceV1::new(RegionEvidenceInputV1 {
                evidence,
                capture_sha256: "a".repeat(64),
                locator: format!("synthetic:{index}"),
                statement: "Synthetic regional membership".to_string(),
            })?,
        }));
    }
    Ok(RegionMembershipContractV1::new(
        RegionMembershipContractInputV1 {
            version: 1,
            revision,
            region: RegionKeyV1::parse("synthetic:backup")?,
            jurisdictions: vec![UsJurisdiction::Michigan],
            effective: range,
            assertions,
        },
    )?)
}

fn profile(as_of: &str, region: ContractRefV1) -> TestResult<QueryProfileV1> {
    let contract = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::SourceBackedRegion(region),
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

struct Fixture {
    identity: StoreIdentity,
    profiles: [QueryProfileV1; 2],
    regions: [RegionMembershipContractV1; 2],
    witness: Witness,
}

fn populate(root: &Path, identity: Option<StoreIdentity>) -> TestResult<Fixture> {
    let store = Store::open_with(root, CACHE_BYTES)?;
    let identity = match identity {
        Some(identity) => identity,
        None => StoreIdentity::parse(&"A".repeat(64))?,
    };
    seed_identity(&store, &identity)?;
    let school = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        "Synthetic School",
        normalize_name("Synthetic School"),
        None,
    )
    .0;
    store.append(Table::Schools, &school)?;
    store.journal_done(
        "synthetic-backup",
        "retained",
        &serde_json::json!({"status":"done"}),
    )?;
    let regions = [region(1, 0)?, region(2, 40)?];
    let profiles = [
        profile("2026-10-11", regions[0].reference()?)?,
        profile("2026-10-12", regions[1].reference()?)?,
    ];
    for region in &regions {
        equal(
            store.register_region_contract(&region.reference()?, region)?,
            RegionContractRegistration::Registered(region.reference()?),
        )?;
    }
    for profile in &profiles {
        equal(
            store.register_profile(profile)?,
            ProfileRegistration::Registered(profile.id()?),
        )?;
    }
    ensure(
        serialized_bytes(&regions[1])?.len() > crate::MAX_PROFILE_RECORD_BYTES,
        "backup fixture must exercise multiple regional chunks",
    )?;
    std::fs::write(
        root.join("http/retained.bytes"),
        b"immutable captured source bytes",
    )?;
    std::fs::write(
        root.join("out/historical-artifact.json"),
        b"{\"historical\":true}",
    )?;
    store.flush()?;
    Ok(Fixture {
        identity,
        profiles,
        regions,
        witness: Witness::capture(&store)?,
    })
}

fn seed_identity(store: &Store, identity: &StoreIdentity) -> TestResult {
    let key = Store::journal_key("export-lineage-v1", "store-identity");
    let raw = format!(
        "{{\"at\":\"2020-01-02T03:04:05Z\",\"key\":\"store-identity\",\"payload\":\"{}\"}}",
        identity.as_str()
    );
    let manifest = RunManifest {
        store_identity: identity.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, raw.as_bytes());
    batch.insert(&store.meta, b"run-manifest", serde_json::to_vec(&manifest)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn inject(store: &Store, key: &[u8], bytes: &[u8]) -> TestResult {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.insert(&store.meta, key, bytes);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn remove(store: &Store, key: &[u8]) -> TestResult {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.remove(&store.meta, key);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn descriptor(reference: &ContractRefV1) -> String {
    format!(
        "region-contracts:v1/{}/{}",
        reference.revision(),
        reference.digest()
    )
}

fn chunk(reference: &ContractRefV1, index: usize) -> TestResult<Vec<u8>> {
    Ok(crate::journal::chunk_key(
        descriptor(reference).as_bytes(),
        index,
    )?)
}

fn raw_rows(snapshot: &Snapshot, keyspace: &Keyspace) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    let mut bytes = 0_usize;
    for guard in snapshot
        .prefix(keyspace, b"")
        .take(FIXTURE_ROWS.checked_add(1).ok_or("row budget")?)
    {
        ensure(
            rows.len() < FIXTURE_ROWS,
            "fixture row inventory exceeded fixed bound",
        )?;
        let (key, value) = guard.into_inner()?;
        bytes = bytes
            .checked_add(key.len())
            .and_then(|count| count.checked_add(value.len()))
            .ok_or("byte counter")?;
        ensure(
            bytes <= FIXTURE_BYTES,
            "fixture byte inventory exceeded fixed bound",
        )?;
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
            table_digest: view.tables_digest(&Table::ALL)?,
            entities: raw_rows(&raw, &store.entities)?,
            journal: raw_rows(&raw, &store.journal)?,
            meta: raw_rows(&raw, &store.meta)?,
            receipts: raw_rows(&raw, &store.receipts)?,
        })
    }
}
