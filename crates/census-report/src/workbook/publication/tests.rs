use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, SourceIdentity, SourceNamespace,
    SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn seed(store: &Store, name: &str) -> crate::report::ReportResult<()> {
    let (mut school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name.to_lowercase(), None);
    let evidence = Evidence::parsed(
        SourceRef::new(
            "wiaa_results",
            Some("https://example.test/results".to_string()),
        ),
        "2026-06-01",
    );
    school.evidence.push(evidence.clone());
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), name),
    );
    athlete.evidence.push(evidence);
    store.append(Table::Athletes, &athlete)?;
    Ok(())
}

#[test]
fn a_frozen_input_reopens_after_new_source_rows_and_store_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let archive = directory.path().join("frozen.json");
    let captured;
    {
        let store = Store::open(&root)?;
        seed(&store, "First School")?;
        let dataset = ExportDataset::load(&store)?;
        captured = dataset.lineage.clone();
        dataset.save_frozen(&archive)?;
        seed(&store, "Later School")?;
    }
    let reopened = ExportDataset::reopen_frozen(&archive)?;
    check!(eq; reopened.lineage, captured);
    check!(eq; reopened
        .schools
        .values()
        .map(|school| school.name.as_str())
        .collect::<Vec<_>>(),
    ["First School"]);
    let store = Store::open(&root)?;
    let current = ExportDataset::load(&store)?;
    check!(eq; current.lineage.store_identity, captured.store_identity);
    check!(ne; current.lineage.input_generation, captured.input_generation);
    Ok(())
}

#[test]
fn a_stale_exporter_cannot_replace_a_newer_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let older = ExportDataset::load(&store)?;
    seed(&store, "New School")?;
    let newer = ExportDataset::load(&store)?;
    let options = Options::default();
    let censuses = crate::workbook::Censuses::of(&newer, &store.out_dir());
    let published = crate::workbook::build_from(&newer, &store, &options, &censuses)?;
    let error = match Stage::begin(&older, &store, &options) {
        Err(error) => error,
        Ok(_) => return Err("stale exporter accepted".into()),
    };
    check!(error.to_string().contains("stale or foreign"));
    check!(eq; current_workbook(&store.out_dir().join("publication"))?,
    published);
    Ok(())
}

#[test]
fn a_failed_generation_keeps_the_previous_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let options = Options::default();
    let published = crate::workbook::build(&store, &options)?;
    let dataset = ExportDataset::load(&store)?;
    let stage = Stage::begin(&dataset, &store, &options)?;
    check!(stage.publish(&dataset, &options, |_| Ok(())).is_err());
    check!(eq; current_workbook(&store.out_dir().join("publication"))?,
    published);
    verify_published(&published)?;
    assert_no_temporaries(&store.out_dir().join("publication"))?;
    Ok(())
}

#[test]
fn a_corrupt_sidecar_invalidates_the_whole_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let published = crate::workbook::build(&store, &Options::default())?;
    let sidecar = published
        .parent()
        .ok_or("missing generation directory")?
        .join("recruiting.csv");
    std::fs::write(sidecar, "corrupt")?;
    let error = match verify_published(&published) {
        Err(error) => error,
        Ok(_) => return Err("corrupt publication accepted".into()),
    };
    check!(error
        .to_string()
        .contains("generation artifact mismatch: recruiting.csv"));
    Ok(())
}

#[test]
fn a_changed_dataset_cannot_masquerade_as_the_captured_input() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let mut dataset = ExportDataset::load(&store)?;
    let (school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Injected", "injected", None);
    dataset.schools.insert(id, school);
    let error = match dataset.save_frozen(&directory.path().join("frozen.json")) {
        Err(error) => error,
        Ok(()) => return Err("mutated input accepted".into()),
    };
    check!(matches!(
        error,
        crate::report::ReportError::Invariant { .. }
    ));
    Ok(())
}

fn assert_no_temporaries(root: &Path) -> TestResult {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        check!(
            !name.starts_with(".staging.") && !name.starts_with(".current."),
            "uncommitted publication temporary survived: {name}"
        );
    }
    Ok(())
}

#[test]
fn source_advance_during_render_cannot_switch_the_publication_pointer() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let options = Options::default();
    let published = crate::workbook::build(&store, &options)?;
    let dataset = ExportDataset::load(&store)?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir());
    let stage = Stage::begin(&dataset, &store, &options)?;
    let error = match stage.publish(&dataset, &options, |path| {
        crate::workbook::write_artifacts(path, &dataset, &options, &censuses)?;
        seed(&store, "Arrived During Render")?;
        Ok(())
    }) {
        Err(error) => error,
        Ok(_) => return Err("stale render publication accepted".into()),
    };
    check!(error.to_string().contains("stale or foreign"));
    let root = store.out_dir().join("publication");
    check!(eq; current_workbook(&root)?, published);
    verify_published(&published)?;
    assert_no_temporaries(&root)?;
    Ok(())
}

#[test]
fn an_export_job_keeps_its_input_and_changed_evidence_requires_a_new_export() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed(&store, "First School")?;
    let first = ExportDataset::for_job(&store, "export:first-input")?;
    store.journal_done("unrelated-work", "attempt", &true)?;
    let replay = ExportDataset::for_job(&store, "export:first-input")?;
    check!(eq; first.lineage, replay.lineage);
    seed(&store, "Later School")?;
    let error = match ExportDataset::for_job(&store, "export:first-input") {
        Err(error) => error,
        Ok(_) => return Err("stale export job accepted".into()),
    };
    check!(error.to_string().contains("stale or foreign"));
    let next = ExportDataset::for_job(&store, "export:changed-input")?;
    check!(ne; next.lineage.input_generation,
    first.lineage.input_generation);
    check!(eq; first.schools.len(), 1);
    check!(eq; next.schools.len(), 2);
    let archive = store.out_dir().join("export-inputs").join(format!(
        "{}.json",
        census_domain::model::serialized_digest(&"export:first-input")?
    ));
    let retained = ExportDataset::reopen_frozen(&archive)?;
    check!(eq; retained.lineage, first.lineage);
    Ok(())
}

#[test]
fn revision_two_frozen_inputs_are_preserved_but_not_reused_under_new_projection_policy(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed(&store, "Captured School")?;
    let archive = directory.path().join("revision-two.json");
    ExportDataset::load(&store)?.save_frozen(&archive)?;
    let mut historical: serde_json::Value = serde_json::from_slice(&std::fs::read(&archive)?)?;
    historical["lineage"]["policy_revision"] = serde_json::json!(2);
    let captured = serde_json::to_vec(&historical)?;
    std::fs::write(&archive, &captured)?;
    check!(matches!(
        ExportDataset::reopen_frozen(&archive),
        Err(crate::report::ReportError::Invariant { .. })
    ));
    check!(eq; std::fs::read(&archive)?, captured);
    Ok(())
}
