use super::{inspect, JobError};
use crate::restate_services::jobs::selected_fixture::{self, Controlled};
use crate::restate_services::national::publication_tests::fixtures;
use crate::restate_services::wire::{BindRunRequest, NationalRequest};
use census_domain::model::{GradYear, ProfileSelectionV1, QueryProfileV1, RunManifest};
use census_report::export::ExportJobRequest;
use census_store::{Store, Table};

mod guards;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type State = (u64, u64, u64, u64, String, Option<RunManifest>);

fn controlled() -> TestResult<Controlled> {
    selected_fixture::install().map_err(|error| -> Box<dyn std::error::Error> { error })
}

fn request(controlled: &Controlled) -> TestResult<NationalRequest> {
    fixtures::selected_request(controlled, &census_domain::UsJurisdiction::CENSUS_SCOPE)
}

fn select(store: &Store, profile: &QueryProfileV1, request: &mut NationalRequest) -> TestResult {
    store.register_profile(profile)?;
    request.export_job = ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(store.store_identity()?, profile.id()?),
    };
    request.jurisdictions = profile.input().jurisdictions.clone();
    Ok(())
}

fn bind(store: &Store, request: &NationalRequest) -> TestResult {
    let reply = crate::restate_services::census::binding::bind(
        store,
        BindRunRequest {
            season: request.season.get(),
            revision: request.revision.get(),
            jurisdictions: request.jurisdictions.clone(),
            selection: Some(request.export_job.require_selected_publication().clone()),
        },
    )?;
    check!(eq; reply.store_identity.as_str(), store.store_identity()?.as_str());
    check!(eq; reply.cohort, GradYear::CO2027.get());
    Ok(())
}

fn state(store: &Store) -> TestResult<State> {
    Ok((
        store.snapshot().sequence(),
        store.receipt_count()?,
        store.evidence_generation(),
        store.derived_generation(),
        store.snapshot().tables_digest(&Table::ALL)?,
        store.run_manifest()?,
    ))
}

fn refuses(store: &Store, request: &NationalRequest, expected: &str) -> TestResult {
    let before = state(store)?;
    let profile = store.selected_profile(request.export_job.require_selected_publication())?;
    let result = inspect(store, request, GradYear::CO2027);
    check!(matches!(result, Err(JobError::Terminal { message }) if message == expected));
    check!(eq; state(store)?, before);
    check!(eq; store.selected_profile(request.export_job.require_selected_publication())?, profile);
    Ok(())
}
