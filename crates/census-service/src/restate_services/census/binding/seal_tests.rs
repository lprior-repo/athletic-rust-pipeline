use super::bind;
use crate::census::seal::{self, SealRequest};
use crate::restate_services::jobs::selected_fixture;
use crate::restate_services::jobs::workbook_job;
use crate::restate_services::wire::BindRunRequest;
use census_domain::model::{CensusRun, GradYear, ProfileSelectionV1, QueryProfileV1, SchoolYear};
use census_domain::UsJurisdiction;
use census_report::report::Scope;
use census_report::workbook::publication::open_publication_for_seal;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request(selection: Option<ProfileSelectionV1>) -> BindRunRequest {
    BindRunRequest {
        season: 2026,
        revision: 1,
        jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
        selection,
    }
}

#[test]
fn selected_cohort_2026_workbook_and_seal_lifecycle() -> TestResult {
    let controlled = selected_fixture::install().map_err(|error| error.to_string())?;
    let store = controlled.store();

    let original = controlled.profile().clone();
    let mut input = original.input().clone();
    input.graduation_years = vec![GradYear::new(2026).ok_or("CO2026 cohort")?];
    let graduation_2026 = QueryProfileV1::new(input)?;
    store.register_profile(&graduation_2026)?;

    let selection = ProfileSelectionV1::new(store.store_identity()?, graduation_2026.id()?);

    let bound = bind(store, request(Some(selection.clone())))?;
    check!(eq; bound.cohort, 2026);

    let options = census_report::workbook::options_for_selected_request(
        store,
        &selection,
        &census_report::workbook::Options {
            address_corpus: None,
            grad_year: None,
            out: None,
            limit: None,
            scope: Scope::AllSources,
            jurisdictions: Vec::new(),
            school_year: SchoolYear::new(2026).ok_or("school year")?,
        },
    )?;
    let export_job = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: selection.clone(),
    };
    let reply = workbook_job::build(store, &options, "seal-tests-2026", &export_job)?;
    check!(eq; reply.grad_year, Some(2026));

    let opened = open_publication_for_seal(
        Path::new(&reply.path),
        store,
        Scope::AllSources,
        GradYear::new(2026).ok_or("CO2026 cohort")?,
        &UsJurisdiction::CENSUS_SCOPE,
    )?;
    check!(eq; opened.dataset.lineage.cohort, GradYear::new(2026));

    let manifest = store.run_manifest()?.ok_or("manifest absent")?;
    check!(eq; manifest.cohort.get(), 2026);

    let run = CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, 1).ok_or("run")?;

    let wrong_cohort = seal::seal(
        store,
        &SealRequest {
            grad_year: 2027,
            scope: Scope::AllSources,
            jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
            workbook: Some(Path::new(&reply.path).to_path_buf()),
            write: true,
            journal: None,
            source_failures: None,
            run: Some(run),
        },
    );
    check!(matches!(
        wrong_cohort,
        Err(seal::SealWorkflowError::CohortMismatch {
            requested: 2027,
            bound: 2026
        })
    ));
    let retained_after_mismatch = store.run_manifest()?.ok_or("manifest absent")?;
    check!(eq; retained_after_mismatch, manifest);

    let outcome = seal::seal(
        store,
        &SealRequest {
            grad_year: 2026,
            scope: Scope::AllSources,
            jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
            workbook: Some(Path::new(&reply.path).to_path_buf()),
            write: true,
            journal: Some(seal::JournalCounts {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(0),
                silent_sources: Vec::new(),
            }),
            source_failures: None,
            run: Some(run),
        },
    )?;
    check!(outcome.refusal.is_some());
    check!(outcome
        .evidence
        .open_items()
        .contains(&crate::census::AcceptanceItem::RetriesRepresented));
    check!(outcome.recorded.is_none());
    check!(outcome.wrote.is_none());
    check!(outcome.state.sealed().is_none());
    check!(!store.out_dir().join("seal.json").exists());

    check!(eq; store.run_manifest()?, Some(manifest));
    check!(eq; store.snapshot().selected_profile(&controlled.selection().map_err(|error| error.to_string())?)?, original);
    Ok(())
}
