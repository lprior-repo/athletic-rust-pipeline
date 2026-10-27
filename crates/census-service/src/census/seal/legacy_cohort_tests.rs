use super::seal;
use census_domain::model::{GradYear, RunManifest};
use census_domain::UsJurisdiction;
use census_report::report::Scope;
use census_store::Store;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn bind_cohort_2026(store: &Store) -> TestResult<()> {
    let identity = store.store_identity()?.as_str().to_owned();
    let run = census_domain::model::CensusRun::new(
        census_domain::model::SchoolYear::new(2026).ok_or("invalid season")?,
        1,
    )
    .ok_or("invalid run")?;
    store.bind_run(&RunManifest {
        store_identity: identity,
        run,
        cohort: GradYear::new(2026).ok_or("legacy cohort")?,
        jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
    })?;
    Ok(())
}

#[test]
fn a_legacy_2026_cohort_is_refused_by_the_current_seal_authority() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    bind_cohort_2026(&store)?;
    let retained = store.run_manifest()?.ok_or("retained legacy binding")?;
    let options = census_report::workbook::Options {
        address_corpus: None,
        grad_year: Some(2026),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("invalid season")?,
    };
    let path = census_report::workbook::build(&store, &options)?;
    let original = std::fs::read(&path)?;
    let request = super::SealRequest {
        grad_year: 2026,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        workbook: Some(path),
        write: true,
        journal: None,
        source_failures: None,
        run: Some(retained.run),
    };
    let error = seal(&store, &request)
        .err()
        .ok_or("expected ForeignCohort")?;
    match error {
        super::SealWorkflowError::ForeignCohort { requested: 2026 } => {}
        other => return Err(format!("unexpected historical-artifact refusal: {other:?}").into()),
    }
    check!(eq; store.run_manifest()?, Some(retained));
    check!(eq; std::fs::read(request.workbook.as_ref().ok_or("workbook path")?)?, original);
    check!(
        !store.out_dir().join("seal.json").exists(),
        "no seal certificate was written for the refused legacy cohort"
    );
    Ok(())
}
