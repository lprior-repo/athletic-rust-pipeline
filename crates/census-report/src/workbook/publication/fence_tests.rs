use super::*;
use crate::export::{ExportDataset, ExportJobRequest};
use crate::workbook::{self, Censuses, Options};
use census_domain::model::{CensusRun, GradYear, RunManifest};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture_options(out: Option<std::path::PathBuf>) -> TestResult<Options> {
    Ok(Options {
        address_corpus: None,
        grad_year: Some(2027),
        out,
        limit: None,
        scope: crate::report::Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
    })
}

#[tokio::test]
async fn publication_fence_rejects_a_changed_expected_root() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let publication_root = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let options = fixture_options(None)?;
    let root = fixture.store.out_dir().join("publication");
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let mut different = options.clone();
    different.out = Some(publication_root.path().join("other"));
    fixture.append_second_meet().await?;
    let other_objects = tempfile::tempdir()?;
    let other_dataset = fixture.capture(other_objects.path())?;
    let other_censuses = Censuses::of(
        &other_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let other = workbook::build_from(&other_dataset, &fixture.store, &different, &other_censuses)?;
    let other_dir = other.parent().ok_or("bundle")?;
    let name = other_dir.file_name().ok_or("digest")?;
    std::fs::rename(other_dir, root.join("generations").join(name))?;
    let selection = fixture.selection.clone();
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let job_dataset = ExportDataset::for_job_request(&fixture.store, "publication-cas", &request)?;
    let stage = Stage::begin(&job_dataset, &fixture.store, &options)?;
    let link = root.join("injected.tmp");
    std::os::unix::fs::symlink(Path::new("generations").join(name), &link)?;
    std::fs::rename(link, root.join("current"))?;
    let job_censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let result = stage.publish(&job_dataset, &options, |path| {
        workbook::write_artifacts(path, &job_dataset, &options, &job_censuses)
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::CurrentRootChanged { .. }
        ))
    ));
    check!(eq; current_workbook(&root)?, root.join("generations").join(name).join("workbook.xlsx"));
    verify_published(&old)?;
    let retry = Stage::begin(&job_dataset, &fixture.store, &options)?;
    check!(eq; retry.expected(), &stage_expected(&old)?);
    let retry_result = retry.publish(&job_dataset, &options, |path| {
        workbook::write_artifacts(path, &job_dataset, &options, &job_censuses)
    });
    check!(matches!(
        retry_result,
        Err(ReportError::Publication(
            PublicationError::CurrentRootChanged { .. }
        ))
    ));
    Ok(())
}

fn stage_expected(workbook: &Path) -> TestResult<census_store::publish::Root> {
    let digest = workbook
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or("digest")?;
    Ok(census_store::publish::Root::Bundle(
        census_store::publish::Digest::parse(digest)?,
    ))
}

#[tokio::test]
async fn publication_fence_run_binding_during_render_refuses_intent() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let options = fixture_options(None)?;
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let bytes = std::fs::read(&old)?;
    let selection = fixture.selection.clone();
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let job_dataset = ExportDataset::for_job_request(&fixture.store, "late-run-binding", &request)?;
    let job_censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let stage = Stage::begin(&job_dataset, &fixture.store, &options)?;
    let result = stage.publish(&job_dataset, &options, |directory| {
        let counts = workbook::write_artifacts(directory, &job_dataset, &options, &job_censuses)?;
        fixture.store.bind_run(&RunManifest {
            store_identity: job_dataset.lineage.store_identity.clone(),
            run: CensusRun::new(options.school_year, 1).ok_or_else(|| invariant("run".into()))?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Wisconsin],
        })?;
        Ok(counts)
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::InputFenceRefused { .. }
        ))
    ));
    let store_root = fixture.store.out_dir().join("publication");
    check!(eq; current_workbook(&store_root)?, old);
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}
