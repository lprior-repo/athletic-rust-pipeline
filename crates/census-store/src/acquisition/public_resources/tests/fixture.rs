use super::super::{codec, read, PublicResourceDeclarationBindingV1};
use super::super::{PublicResourceDeclarationV1, PublicResourceOwnerBindingV1};
use crate::acquisition::{qualification, AcquisitionReadBudget, CaptureWriterV1};
use crate::{Application, Store, StoreError};
use census_domain::model::*;
use census_domain::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
use census_domain::UsJurisdiction;
use chrono::{DateTime, Utc};
use fjall::{Keyspace, Readable, Snapshot};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Rows = Vec<(Vec<u8>, Vec<u8>)>;

pub(super) struct Fixture {
    pub(super) directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) writer: CaptureWriterV1,
    pub(super) binding: PublicResourceDeclarationBindingV1,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        Self::for_sites(vec![UsJurisdiction::Michigan])
    }

    pub(super) fn for_sites(mut sites: Vec<UsJurisdiction>) -> TestResult<Self> {
        sites.sort_unstable();
        sites.dedup();
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        Store::create_current_qualification_v3(&root)?;
        let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        let profile = profile_for_sites("2026-10-14", sites.clone())?;
        store.register_profile(&profile)?;
        let manifest = RunManifest {
            store_identity: writer.store_identity().as_str().to_string(),
            run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
            cohort: GradYear::CO2027,
            jurisdictions: sites.clone(),
        };
        store.bind_run(&manifest)?;
        let capture = qualification::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.store_format()?,
        )?
        .ok_or("capture qualification")?;
        let binding = PublicResourceDeclarationBindingV1 {
            version: 1,
            store: writer.store_identity().clone(),
            run: manifest.run,
            capture_declaration_digest: codec::object_digest(&capture)?,
            run_manifest_digest: codec::object_digest(&manifest)?,
            selection: ProfileSelectionV1::new(writer.store_identity().clone(), profile.id()?),
            national_owner_key: "controlled-national-owner".to_string(),
            national_request_digest: digest("a")?,
            owners: sites
                .into_iter()
                .map(owner)
                .collect::<TestResult<Vec<_>>>()?,
        };
        Ok(Self {
            directory,
            store,
            writer,
            binding,
        })
    }

    pub(super) fn declare(&self, at: &str) -> TestResult<Application> {
        Ok(self.store.declare_public_resource_capture(
            &self.writer,
            self.binding.clone(),
            timestamp(at)?,
        )?)
    }
}

pub(super) fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    profile_for_sites(as_of, vec![UsJurisdiction::Michigan])
}

fn profile_for_sites(as_of: &str, sites: Vec<UsJurisdiction>) -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: sites,
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse(as_of)?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::DEFAULT,
        source_plan: profile_contract_ref_v1(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-13")?,
        )?,
        completion_contract: profile_contract_ref_v1(ContractRoleV1::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: profile_contract_ref_v1(ContractRoleV1::Cohort)?,
            event: profile_contract_ref_v1(ContractRoleV1::Event)?,
            identity: profile_contract_ref_v1(ContractRoleV1::Identity)?,
            affiliation: profile_contract_ref_v1(ContractRoleV1::Affiliation)?,
            contact: profile_contract_ref_v1(ContractRoleV1::Contact)?,
            participation: profile_contract_ref_v1(ContractRoleV1::Participation)?,
            publication: profile_contract_ref_v1(ContractRoleV1::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

fn owner(site: UsJurisdiction) -> TestResult<PublicResourceOwnerBindingV1> {
    let origin = format!("https://{}.milesplit.com", site.code().to_ascii_lowercase());
    Ok(PublicResourceOwnerBindingV1 {
        owner_key: format!("jurisdiction:{}:2026-27:1", site.code()),
        site,
        jurisdiction_request_digest: digest("b")?,
        source_plan_fingerprint: digest("c")?,
        history: HistoryWindow::new(2023, 2026, "2026-10-14")?,
        cache_intent: CaptureCacheIntent::ReuseEligible,
        directory_request: request(&format!("{origin}/teams"))?,
        robots_request: request(&format!("{origin}/robots.txt"))?,
    })
}

fn request(url: &str) -> TestResult<RequestRepresentationV1> {
    Ok(RequestRepresentationV1::normalized_get(
        url,
        vec![
            ("accept".to_string(), "*/*".to_string()),
            ("accept-encoding".to_string(), "identity".to_string()),
            (
                "user-agent".to_string(),
                "Census/store-declaration-test".to_string(),
            ),
        ],
        "Census/store-declaration-test",
    )?)
}

pub(super) fn digest(nibble: &str) -> TestResult<AcquisitionDigest> {
    Ok(AcquisitionDigest::parse(&nibble.repeat(64))?)
}

pub(super) fn different_digest(value: &AcquisitionDigest) -> TestResult<AcquisitionDigest> {
    let first = if value.as_str().starts_with('0') {
        "1"
    } else {
        "0"
    };
    let tail = value.as_str().get(1..).ok_or("digest tail")?;
    Ok(AcquisitionDigest::parse(&format!("{first}{tail}"))?)
}

pub(super) fn timestamp(value: &str) -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

pub(super) fn declaration(store: &Store) -> TestResult<PublicResourceDeclarationV1> {
    Ok(read::declaration(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.receipts,
        &store.store_format()?,
        &mut AcquisitionReadBudget::for_work(),
    )?
    .ok_or("retained public resource declaration")?)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Witness {
    pub(super) sequence: u64,
    pub(super) entities: Rows,
    pub(super) journal: Rows,
    pub(super) meta: Rows,
    pub(super) receipts: Rows,
}

impl Witness {
    pub(super) fn capture(store: &Store) -> TestResult<Self> {
        let snapshot = store.db.snapshot();
        Ok(Self {
            sequence: snapshot.seqno(),
            entities: rows(&snapshot, &store.entities)?,
            journal: rows(&snapshot, &store.journal)?,
            meta: rows(&snapshot, &store.meta)?,
            receipts: rows(&snapshot, &store.receipts)?,
        })
    }
}

fn rows(snapshot: &Snapshot, table: &Keyspace) -> TestResult<Rows> {
    let mut rows = Vec::new();
    for guard in snapshot.prefix(table, b"").take(513) {
        if rows.len() == 512 {
            return Err("declaration fixture inventory exceeds 512 rows".into());
        }
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        rows.push((key.to_vec(), value.to_vec()));
    }
    Ok(rows)
}

pub(super) fn cold_refused(root: &std::path::Path) -> TestResult {
    check!(Store::open_current_qualification_v3(root).is_err());
    check!(Store::open_public_identity_qualification_v3(root).is_err());
    Ok(())
}

pub(super) fn refused<T>(result: crate::StoreResult<T>, expected: &str) -> TestResult {
    match result {
        Err(StoreError::Refused { detail }) if detail == expected => Ok(()),
        Err(error) => Err(format!("expected refusal {expected:?}, received {error}").into()),
        Ok(_) => Err(format!("expected refusal {expected:?}, received success").into()),
    }
}
