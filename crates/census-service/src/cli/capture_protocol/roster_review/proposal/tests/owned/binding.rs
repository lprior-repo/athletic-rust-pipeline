use super::*;
use census_domain::model::*;
use census_domain::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use census_service::restate_services::{
    compute_plan_fingerprint_with_nces_receipt, BrowserLaneState, NationalRequest,
};
use census_store::{
    CaptureWriterV1, PublicResourceDeclarationBindingV1, PublicResourceOwnerBindingV1,
};

pub(super) fn declare(
    store: &Store,
    writer: &CaptureWriterV1,
    fetcher: &Fetcher,
) -> TestResult<()> {
    let profile = profile()?;
    store.register_profile(&profile)?;
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().into(),
        run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("controlled run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    store.bind_run(&manifest)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let national: NationalRequest = serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 1, "jurisdictions": ["MI"],
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-14"},
        "publication_contract": "selected_publication_v2",
        "export_job": {"contract": "selected_publication_v1", "selection": selection}
    }))?;
    let request = national.for_jurisdiction(UsJurisdiction::Michigan);
    let options = FetchOptions::default();
    store.declare_public_resource_capture(
        writer,
        PublicResourceDeclarationBindingV1 {
            version: 1,
            store: store.store_identity()?,
            run: manifest.run,
            capture_declaration_digest: writer.public_resource_capture_digest(store)?,
            run_manifest_digest: digest(&manifest)?,
            selection: national.export_job.require_selected_publication().clone(),
            national_owner_key: WorkflowIdentity::national(
                national.season,
                national.revision,
                &national.jurisdictions,
            )
            .to_string(),
            national_request_digest: digest(&national)?,
            owners: vec![PublicResourceOwnerBindingV1 {
                owner_key: WorkflowIdentity::jurisdiction(
                    request.jurisdiction,
                    request.season,
                    request.revision,
                )
                .to_string(),
                site: request.jurisdiction,
                jurisdiction_request_digest: digest(&request)?,
                source_plan_fingerprint: AcquisitionDigest::parse(
                    &compute_plan_fingerprint_with_nces_receipt(
                        request.jurisdiction,
                        request.season,
                        request.revision,
                        BrowserLaneState::Absent,
                        None,
                    ),
                )?,
                history: request.history,
                cache_intent: CaptureCacheIntent::ReuseEligible,
                directory_request: fetcher
                    .public_resource_request("https://mi.milesplit.com/teams", &options)?,
                robots_request: fetcher
                    .public_resource_request("https://mi.milesplit.com/robots.txt", &options)?,
            }],
        },
        chrono::DateTime::parse_from_rfc3339("2026-10-01T12:00:00Z")?.to_utc(),
    )?;
    Ok(())
}

fn digest(value: &impl serde::Serialize) -> TestResult<AcquisitionDigest> {
    Ok(AcquisitionDigest::parse(&serialized_digest(value)?)?)
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
