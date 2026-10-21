use super::*;
use census_domain::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
use census_store::{
    CaptureWriterV1, PublicResourceCaptureOwnerV1, PublicResourceDeclarationBindingV1,
    PublicResourceOwnerBindingV1, PublicResourceRootKindV1, Store,
};
use std::collections::HashMap;
use std::time::Duration;

mod retention;

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) writer: CaptureWriterV1,
    pub(super) owner: PublicResourceCaptureOwnerV1,
    pub(super) captured: CapturedFetch,
    pub(super) reference: AcquisitionOccurrenceRefV1,
}

pub(super) fn fixture(body: &[u8], status: u16) -> TestResult<Fixture> {
    let root = tempfile::tempdir()?;
    Store::create_current_qualification_v3(root.path().join("store"))?;
    let (store, writer) =
        Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
    let fetcher = crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?;
    declare(&store, &writer, &fetcher)?;
    let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
    let reference = retention::retain(&store, &writer, &owner, body, status)?;
    let held = store.read_retained_acquisition_v1(
        &reference,
        &mut census_store::AcquisitionReadBudget::for_work(),
    )?;
    let captured = crate::net::acquisition::read::outcome(held, false)?;
    Ok(Fixture {
        root,
        store,
        writer,
        owner,
        captured,
        reference,
    })
}

impl Fixture {
    pub(super) fn evidence(&self) -> TestResult<PublicDirectoryEvidenceV1> {
        Ok(self.owner.directory_evidence(
            &self.store,
            &self.reference,
            &mut census_store::AcquisitionReadBudget::for_work(),
        )?)
    }

    pub(super) fn reopen(self) -> TestResult<Self> {
        let Self {
            root,
            store,
            writer,
            owner,
            captured,
            reference,
        } = self;
        store.flush()?;
        drop((owner, writer, store, captured));
        let (store, writer) =
            Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
        let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
        let held = store.read_retained_acquisition_v1(
            &reference,
            &mut census_store::AcquisitionReadBudget::for_work(),
        )?;
        let captured = crate::net::acquisition::read::outcome(held, true)?;
        Ok(Self {
            root,
            store,
            writer,
            owner,
            captured,
            reference,
        })
    }
}

fn declare(store: &Store, writer: &CaptureWriterV1, fetcher: &crate::net::Fetcher) -> TestResult {
    let profile = profile()?;
    store.register_profile(&profile)?;
    let manifest = RunManifest {
        store_identity: writer.store_identity().as_str().to_owned(),
        run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("controlled run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    store.bind_run(&manifest)?;
    let qualification = CaptureQualificationV2::new(writer.store_identity().clone());
    let binding = PublicResourceDeclarationBindingV1 {
        version: 1,
        store: writer.store_identity().clone(),
        run: manifest.run,
        capture_declaration_digest: AcquisitionDigest::parse(&serialized_digest(&qualification)?)?,
        run_manifest_digest: AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
        selection: ProfileSelectionV1::new(writer.store_identity().clone(), profile.id()?),
        national_owner_key: "controlled-directory-witness".into(),
        national_request_digest: AcquisitionDigest::parse(&"a".repeat(64))?,
        owners: vec![PublicResourceOwnerBindingV1 {
            owner_key: "jurisdiction:MI:2026-27:1".into(),
            site: UsJurisdiction::Michigan,
            jurisdiction_request_digest: AcquisitionDigest::parse(&"b".repeat(64))?,
            source_plan_fingerprint: AcquisitionDigest::parse(&"c".repeat(64))?,
            history: HistoryWindow::new(2023, 2026, "2026-10-14")?,
            cache_intent: CaptureCacheIntent::ReuseEligible,
            directory_request: fetcher
                .public_resource_request(URL, &crate::net::FetchOptions::default())?,
            robots_request: fetcher.public_resource_request(
                "https://mi.milesplit.com/robots.txt",
                &crate::net::FetchOptions::default(),
            )?,
        }],
    };
    let admitted =
        chrono::DateTime::parse_from_rfc3339("2026-10-14T04:59:59Z")?.with_timezone(&chrono::Utc);
    check!(store
        .declare_public_resource_capture(writer, binding, admitted)?
        .written());
    Ok(())
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse("2026-10-14")?,
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
