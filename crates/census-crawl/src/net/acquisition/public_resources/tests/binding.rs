use super::*;
use census_domain::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
use census_store::{
    CaptureWriterV1, PublicResourceDeclarationBindingV1, PublicResourceOwnerBindingV1,
};

pub(super) fn declare(
    store: &Store,
    writer: &CaptureWriterV1,
    fetcher: &Fetcher,
    robots: &FetchOptions,
) -> TestResult<PublicResourceDeclarationBindingV1> {
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
        national_owner_key: "controlled-public-resource-transport".into(),
        national_request_digest: AcquisitionDigest::parse(&"a".repeat(64))?,
        owners: vec![PublicResourceOwnerBindingV1 {
            owner_key: "jurisdiction:MI:2026-27:1".into(),
            site: UsJurisdiction::Michigan,
            jurisdiction_request_digest: AcquisitionDigest::parse(&"b".repeat(64))?,
            source_plan_fingerprint: AcquisitionDigest::parse(&"c".repeat(64))?,
            history: HistoryWindow::new(2023, 2026, "2026-10-14")?,
            cache_intent: CaptureCacheIntent::ReuseEligible,
            directory_request: fetcher
                .public_resource_request(&format!("{ORIGIN}/teams"), &FetchOptions::default())?,
            robots_request: fetcher
                .public_resource_request(&format!("{ORIGIN}/robots.txt"), robots)?,
        }],
    };
    let admitted = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(3))
        .ok_or("controlled declaration clock")?;
    let application = store.declare_public_resource_capture(writer, binding.clone(), admitted)?;
    check!(application.written());
    Ok(binding)
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
