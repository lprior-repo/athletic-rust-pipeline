use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn options(out: Option<PathBuf>) -> Result<Options, Box<dyn std::error::Error>> {
    Ok(Options {
        grad_year: Some(2027),
        out,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("season")?,
        jurisdictions: Vec::new(),
    })
}

#[test]
fn publication_fence_rejects_a_changed_expected_root() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let root = store.out_dir().join("publication");
    let mut different = options.clone();
    different.out = Some(temp.path().join("other"));
    different.limit = Some(1);
    let other = crate::workbook::build(&store, &different)?;
    let other_dir = other.parent().ok_or("bundle")?;
    let name = other_dir.file_name().ok_or("digest")?;
    std::fs::rename(other_dir, root.join("generations").join(name))?;
    let dataset = ExportDataset::for_job(&store, "publication-cas")?;
    let stage = Stage::begin(&dataset, &store, &options)?;
    let link = root.join("injected.tmp");
    std::os::unix::fs::symlink(Path::new("generations").join(name), &link)?;
    std::fs::rename(link, root.join("current"))?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[])?;
    let result = stage.publish(&dataset, &options, |path| {
        crate::workbook::write_artifacts(path, &dataset, &options, &censuses)
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::CurrentRootChanged { .. }
        ))
    ));
    check!(eq; current_workbook(&root)?, root.join("generations").join(name).join("workbook.xlsx"));
    verify_published(&old)?;
    let retry = Stage::begin(&dataset, &store, &options)?;
    check!(eq; retry.expected(), &stage_expected(&old)?);
    let retry_result = retry.publish(&dataset, &options, |path| {
        crate::workbook::write_artifacts(path, &dataset, &options, &censuses)
    });
    check!(matches!(
        retry_result,
        Err(ReportError::Publication(
            PublicationError::CurrentRootChanged { .. }
        ))
    ));
    Ok(())
}

fn stage_expected(
    workbook: &Path,
) -> Result<census_store::publish::Root, Box<dyn std::error::Error>> {
    let digest = workbook
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or("digest")?;
    Ok(census_store::publish::Root::Bundle(
        census_store::publish::Digest::parse(digest)?,
    ))
}

#[test]
fn publication_fence_run_binding_during_render_refuses_intent() -> TestResult {
    use census_domain::model::{CensusRun, GradYear, RunManifest};
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let bytes = std::fs::read(&old)?;
    let dataset = ExportDataset::for_job(&store, "late-run-binding")?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[])?;
    let stage = Stage::begin(&dataset, &store, &options)?;
    let result = stage.publish(&dataset, &options, |directory| {
        let counts = crate::workbook::write_artifacts(directory, &dataset, &options, &censuses)?;
        store.bind_run(&RunManifest {
            store_identity: dataset.lineage.store_identity.clone(),
            run: CensusRun::new(options.school_year, 1).ok_or_else(|| invariant("run".into()))?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![census_domain::UsJurisdiction::Wisconsin],
        })?;
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
