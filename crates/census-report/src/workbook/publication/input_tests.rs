use super::*;
use crate::export::{ExportDataset, ExportJobRequest};
use crate::workbook::{self, Censuses, Options};
use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture_options(out: Option<std::path::PathBuf>) -> TestResult<Options> {
    Ok(Options {
        grad_year: Some(2027),
        out,
        limit: None,
        scope: crate::report::Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
    })
}

#[tokio::test]
async fn publication_fence_evidence_changed_during_render_refuses_intent() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let options = fixture_options(None)?;
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let bytes = std::fs::read(&old)?;
    let selection = fixture.selection.clone();
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let job_dataset =
        ExportDataset::for_job_request(&fixture.store, "evidence-during-render", &request)?;
    let job_censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let stage = Stage::begin(&job_dataset, &fixture.store, &options)?;
    let result = stage.publish(&job_dataset, &options, |directory| {
        let counts = workbook::write_artifacts(directory, &job_dataset, &options, &job_censuses)?;
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Late School",
            "late-school",
            None,
        );
        fixture.store.append(Table::Schools, &school)?;
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

#[tokio::test]
async fn publication_fence_changed_recipe_cannot_receipt_another_bundle() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let options = fixture_options(None)?;
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let bytes = std::fs::read(&old)?;
    let selection = fixture.selection.clone();
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let job_dataset = ExportDataset::for_job_request(&fixture.store, "changed-recipe", &request)?;
    let stage = Stage::begin(&job_dataset, &fixture.store, &options)?;
    let mut changed = options.clone();
    changed.limit = Some(1);
    let job_censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &changed.jurisdictions,
    )?;
    let result = stage.publish(&job_dataset, &changed, |directory| {
        workbook::write_artifacts(directory, &job_dataset, &changed, &job_censuses)
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

#[tokio::test]
async fn publication_fence_stale_snapshot_is_typed_input_refusal() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let options = fixture_options(None)?;
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let bytes = std::fs::read(&old)?;
    let selection = fixture.selection.clone();
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let job_dataset =
        ExportDataset::for_job_request(&fixture.store, "stale-before-stage", &request)?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "New Evidence",
        "new-evidence",
        None,
    );
    fixture.store.append(Table::Schools, &school)?;
    let job_censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let result = workbook::build_from(&job_dataset, &fixture.store, &options, &job_censuses);
    match &result {
        Err(ReportError::Publication(PublicationError::InputFenceRefused { .. })) => {}
        other => return Err(format!("expected typed input refusal, got {other:?}").into()),
    }
    let store_root = fixture.store.out_dir().join("publication");
    check!(eq; current_workbook(&store_root)?, old);
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

#[tokio::test]
async fn publication_fence_visible_requested_bundle_still_fences_new_intent() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let publication_root = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let options = fixture_options(None)?;
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let bytes = std::fs::read(&old)?;
    let selection = fixture.selection.clone();
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    let job_dataset =
        ExportDataset::for_job_request(&fixture.store, "visible-requested", &request)?;
    let job_censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let mut other = options.clone();
    other.out = Some(publication_root.path().join("other"));
    let workbook = workbook::build_from(&job_dataset, &fixture.store, &other, &job_censuses)?;
    let directory = workbook.parent().ok_or("bundle")?;
    let desired = census_store::publish::Digest::parse(
        directory
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("digest")?,
    )?;
    let store_root = fixture.store.out_dir().join("publication");
    std::fs::rename(
        directory,
        store_root.join("generations").join(desired.as_str()),
    )?;
    pointer::switch(&store_root, &desired)?;
    sync_directory(&store_root)?;
    let stage = Stage::begin(&job_dataset, &fixture.store, &options)?;
    let result = stage.publish(&job_dataset, &options, |directory| {
        stage_artifacts(
            &store_root.join("generations").join(desired.as_str()),
            directory,
        )?;
        let counts =
            manifest::retained_counts(&store_root.join("generations").join(desired.as_str()))?;
        let captured =
            manifest::Manifest::capture(directory, &job_dataset, &options, counts.clone())?;
        if captured.generation_digest != desired.as_str() {
            return Err(invariant(
                "fixture must render the already-visible bundle".into(),
            ));
        }
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Late School",
            "late-school",
            None,
        );
        fixture.store.append(Table::Schools, &school)?;
        Ok(counts)
    });
    match &result {
        Err(ReportError::Publication(PublicationError::InputFenceRefused { .. })) => {}
        other => return Err(format!("new Intent must be input-fenced, got {other:?}").into()),
    }
    check!(eq; pointer::read(&store_root)?, census_store::publish::Root::Bundle(desired));
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

fn stage_artifacts(source: &Path, destination: &Path) -> ReportResult<()> {
    let mut stack = vec![(source.to_path_buf(), destination.to_path_buf())];
    let mut staged = 0usize;
    while let Some((from_dir, to_dir)) = stack.pop() {
        for entry in std::fs::read_dir(&from_dir).map_err(|error| io_error(&from_dir, error))? {
            let entry = entry.map_err(|error| io_error(&from_dir, error))?;
            if from_dir.as_path() == source && entry.file_name() == "manifest.json" {
                continue;
            }
            let target = to_dir.join(entry.file_name());
            if entry
                .file_type()
                .map_err(|error| io_error(&from_dir, error))?
                .is_dir()
            {
                std::fs::create_dir(&target).map_err(|error| io_error(&target, error))?;
                stack.push((entry.path(), target));
            } else {
                std::fs::hard_link(entry.path(), &target)
                    .map_err(|error| io_error(&target, error))?;
            }
            staged = staged.checked_add(1).ok_or_else(|| {
                io_error(
                    &entry.path(),
                    std::io::Error::new(std::io::ErrorKind::QuotaExceeded, "stage budget"),
                )
            })?;
            if staged > 65_536 {
                return Err(io_error(
                    &entry.path(),
                    std::io::Error::new(std::io::ErrorKind::QuotaExceeded, "stage entry budget"),
                ));
            }
        }
    }
    Ok(())
}
