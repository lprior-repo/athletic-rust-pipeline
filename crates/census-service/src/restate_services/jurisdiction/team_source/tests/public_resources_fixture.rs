use crate::restate_services::plan::{self, BrowserLaneState};
use crate::restate_services::wire::{NationalRequest, TeamsSourceRequest};
use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::model::*;
use census_domain::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
use census_reconcile::identity::WorkflowIdentity;
use census_store::{CaptureWriterV1, PublicResourceDeclarationBindingV1};
use census_store::{PublicResourceOwnerBindingV1, Store};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn national(store: &Store, source: &TeamsSourceRequest) -> TestResult<NationalRequest> {
    let profile = profile(source)?;
    let mut national = super::bound_national_request(source)?;
    national.export_job = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(store.store_identity()?, profile.id()?),
    };
    Ok(national)
}

pub(super) fn bind(store: &Store, source: &TeamsSourceRequest) -> TestResult<()> {
    store.register_profile(&profile(source)?)?;
    store.bind_run(&RunManifest {
        store_identity: store.store_identity()?.as_str().into(),
        run: CensusRun::new(
            source.jurisdiction.season,
            source.jurisdiction.revision.get(),
        )
        .ok_or("qualified source run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![source.jurisdiction.jurisdiction],
    })?;
    Ok(())
}

pub(super) fn fingerprint(source: &TeamsSourceRequest) -> String {
    plan::compute_plan_fingerprint(
        source.jurisdiction.jurisdiction,
        source.jurisdiction.season,
        source.jurisdiction.revision,
        BrowserLaneState::Absent,
    )
}

pub(super) fn declare(
    store: &Store,
    writer: &CaptureWriterV1,
    source: &TeamsSourceRequest,
    fetcher: &Fetcher,
) -> TestResult<()> {
    let national = national(store, source)?;
    let manifest = store.run_manifest()?.ok_or("qualified source manifest")?;
    let request = &source.jurisdiction;
    let site = census_crawl::milesplit::Site::for_jurisdiction(request.jurisdiction);
    let options = FetchOptions {
        refresh: request.refresh,
        ..FetchOptions::default()
    };
    let binding = PublicResourceDeclarationBindingV1 {
        version: 1,
        store: writer.store_identity().clone(),
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
            jurisdiction_request_digest: digest(request)?,
            source_plan_fingerprint: AcquisitionDigest::parse(&fingerprint(source))?,
            history: request.history,
            cache_intent: if request.refresh {
                CaptureCacheIntent::AcquireNew
            } else {
                CaptureCacheIntent::ReuseEligible
            },
            directory_request: fetcher.public_resource_request(&site.teams_url(), &options)?,
            robots_request: fetcher.public_resource_request(
                &format!("https://{}/robots.txt", site.host()),
                &options,
            )?,
        }],
    };
    store.declare_public_resource_capture(writer, binding, chrono::Utc::now())?;
    Ok(())
}

fn digest(value: &impl serde::Serialize) -> TestResult<AcquisitionDigest> {
    Ok(AcquisitionDigest::parse(&serialized_digest(value)?)?)
}

fn profile(source: &TeamsSourceRequest) -> TestResult<QueryProfileV1> {
    let request = &source.jurisdiction;
    let as_of = ProfileDate::parse(&request.history.as_of().to_string())?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![request.jurisdiction],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: as_of.clone(),
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: request.season,
        source_plan: profile_contract_ref_v1(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(as_of.clone(), as_of)?,
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
