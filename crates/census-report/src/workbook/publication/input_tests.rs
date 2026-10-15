use super::*;
use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_store::Table;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn publication_fence_evidence_changed_during_render_refuses_intent() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let bytes = std::fs::read(&old)?;
    let dataset = ExportDataset::for_job(&store, "evidence-during-render")?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let stage = Stage::begin(&dataset, &store, &options)?;
    let result = stage.publish(&dataset, &options, |directory| {
        let counts = crate::workbook::write_artifacts(directory, &dataset, &options, &censuses)?;
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Late School",
            "late-school",
            None,
        );
        store.append(Table::Schools, &school)?;
        Ok(counts)
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::InputFenceRefused { .. }
        ))
    ));
    let root = store.out_dir().join("publication");
    check!(eq; current_workbook(&root)?, old);
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

#[test]
fn publication_fence_changed_recipe_cannot_receipt_another_bundle() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let bytes = std::fs::read(&old)?;
    let dataset = ExportDataset::for_job(&store, "changed-recipe")?;
    let stage = Stage::begin(&dataset, &store, &options)?;
    let mut changed = options;
    changed.limit = Some(1);
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let result = stage.publish(&dataset, &changed, |directory| {
        crate::workbook::write_artifacts(directory, &dataset, &changed, &censuses)
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::InputFenceRefused { .. }
        ))
    ));
    let root = store.out_dir().join("publication");
    check!(eq; current_workbook(&root)?, old);
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

#[test]
fn publication_fence_stale_snapshot_is_typed_input_refusal() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let bytes = std::fs::read(&old)?;
    let dataset = ExportDataset::for_job(&store, "stale-before-stage")?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "New Evidence",
        "new-evidence",
        None,
    );
    store.append(Table::Schools, &school)?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let result = crate::workbook::build_from(&dataset, &store, &options, &censuses);
    match &result {
        Err(ReportError::Publication(PublicationError::InputFenceRefused { .. })) => {}
        other => return Err(format!("expected typed input refusal, got {other:?}").into()),
    }
    let root = store.out_dir().join("publication");
    check!(eq; current_workbook(&root)?, old);
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

#[test]
fn publication_fence_visible_requested_bundle_still_fences_new_intent() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let bytes = std::fs::read(&old)?;
    let dataset = ExportDataset::for_job(&store, "visible-requested")?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let mut other = options.clone();
    other.out = Some(temp.path().join("other"));
    let workbook = crate::workbook::build_from(&dataset, &store, &other, &censuses)?;
    let directory = workbook.parent().ok_or("bundle")?;
    let desired = census_store::publish::Digest::parse(
        directory
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("digest")?,
    )?;
    let root = store.out_dir().join("publication");
    std::fs::rename(directory, root.join("generations").join(desired.as_str()))?;
    pointer::switch(&root, &desired)?;
    sync_directory(&root)?;
    let stage = Stage::begin(&dataset, &store, &options)?;
    let result = stage.publish(&dataset, &options, |directory| {
        stage_artifacts(&root.join("generations").join(desired.as_str()), directory)?;
        let counts = manifest::retained_counts(&root.join("generations").join(desired.as_str()))?;
        let captured = manifest::Manifest::capture(directory, &dataset, &options, counts.clone())?;
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
        store.append(Table::Schools, &school)?;
        Ok(counts)
    });
    match &result {
        Err(ReportError::Publication(PublicationError::InputFenceRefused { .. })) => {}
        other => return Err(format!("new Intent must be input-fenced, got {other:?}").into()),
    }
    check!(eq; pointer::read(&root)?, census_store::publish::Root::Bundle(desired));
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

fn stage_artifacts(source: &Path, destination: &Path) -> ReportResult<()> {
    for entry in std::fs::read_dir(source).map_err(|error| io_error(source, error))? {
        let entry = entry.map_err(|error| io_error(source, error))?;
        if entry.file_name() == "manifest.json" {
            continue;
        }
        let target = destination.join(entry.file_name());
        std::fs::hard_link(entry.path(), &target).map_err(|error| io_error(&target, error))?;
    }
    Ok(())
}
