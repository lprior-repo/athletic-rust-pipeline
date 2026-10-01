use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, SourceIdentity, SourceNamespace,
    SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::Table;

fn seed(store: &Store, name: &str) {
    let (mut school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name.to_lowercase());
    let evidence = Evidence::parsed(
        SourceRef::new(
            "wiaa_results",
            Some("https://example.test/results".to_string()),
        ),
        "2026-06-01",
    );
    school.evidence.push(evidence.clone());
    store
        .append(Table::Schools, &school)
        .expect("persist school");
    let mut athlete = CanonicalAthlete::new(
        &id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), name),
    );
    athlete.evidence.push(evidence);
    store
        .append(Table::Athletes, &athlete)
        .expect("persist athlete");
}

#[test]
fn a_frozen_input_reopens_after_new_source_rows_and_store_reopen() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let root = directory.path().join("store");
    let archive = directory.path().join("frozen.json");
    let captured;
    {
        let store = Store::open(&root).expect("own store");
        seed(&store, "First School");
        let dataset = ExportDataset::load(&store).expect("capture input");
        captured = dataset.lineage.clone();
        dataset.save_frozen(&archive).expect("freeze input");
        seed(&store, "Later School");
    }
    let reopened = ExportDataset::reopen_frozen(&archive).expect("reopen frozen input");
    assert_eq!(reopened.lineage, captured);
    assert_eq!(
        reopened
            .schools
            .values()
            .map(|school| school.name.as_str())
            .collect::<Vec<_>>(),
        ["First School"]
    );
    let store = Store::open(&root).expect("reopen store");
    let current = ExportDataset::load(&store).expect("capture current input");
    assert_eq!(current.lineage.store_identity, captured.store_identity);
    assert_ne!(current.lineage.input_generation, captured.input_generation);
}

#[test]
fn a_stale_exporter_cannot_replace_a_newer_generation() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let older = ExportDataset::load(&store).expect("capture old input");
    seed(&store, "New School");
    let newer = ExportDataset::load(&store).expect("capture new input");
    let options = Options::default();
    let censuses = crate::workbook::Censuses::of(&newer, &store.out_dir());
    let published = crate::workbook::build_from(&newer, &store, &options, &censuses)
        .expect("publish new generation");
    let error = Stage::begin(&older, &store, &options)
        .err()
        .expect("stale exporter refused");
    assert!(error.to_string().contains("stale or foreign"));
    assert_eq!(
        current_workbook(&store.out_dir().join("publication")).expect("resolve current"),
        published
    );
}

#[test]
fn a_failed_generation_keeps_the_previous_publication() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let options = Options::default();
    let published = crate::workbook::build(&store, &options).expect("publish initial generation");
    let dataset = ExportDataset::load(&store).expect("capture input");
    let stage = Stage::begin(&dataset, &store, &options).expect("start replacement");
    assert!(stage.publish(&dataset, &options, |_| Ok(())).is_err());
    assert_eq!(
        current_workbook(&store.out_dir().join("publication")).expect("resolve current"),
        published
    );
    verify_published(&published).expect("previous generation remains readable");
    assert_no_temporaries(&store.out_dir().join("publication"));
}

#[test]
fn a_corrupt_sidecar_invalidates_the_whole_generation() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let published =
        crate::workbook::build(&store, &Options::default()).expect("publish generation");
    let sidecar = published
        .parent()
        .expect("generation directory")
        .join("recruiting.csv");
    std::fs::write(sidecar, "corrupt").expect("inject artifact corruption");
    let error = verify_published(&published).expect_err("corrupt bundle refused");
    assert!(error
        .to_string()
        .contains("generation artifact mismatch: recruiting.csv"));
}

#[test]
fn a_changed_dataset_cannot_masquerade_as_the_captured_input() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let mut dataset = ExportDataset::load(&store).expect("capture input");
    let (school, id) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Injected", "injected");
    dataset.schools.insert(id, school);
    let error = dataset
        .save_frozen(&directory.path().join("frozen.json"))
        .expect_err("mutated input refused");
    assert!(matches!(
        error,
        crate::report::ReportError::Invariant { .. }
    ));
}

fn assert_no_temporaries(root: &Path) {
    for entry in std::fs::read_dir(root).expect("publication directory") {
        let entry = entry.expect("publication entry");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.starts_with(".staging.") && !name.starts_with(".current."),
            "uncommitted publication temporary survived: {name}"
        );
    }
}

#[test]
fn source_advance_during_render_cannot_switch_the_publication_pointer() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let options = Options::default();
    let published = crate::workbook::build(&store, &options).expect("initial publication");
    let dataset = ExportDataset::load(&store).expect("capture input");
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir());
    let stage = Stage::begin(&dataset, &store, &options).expect("start generation");
    let error = stage
        .publish(&dataset, &options, |path| {
            crate::workbook::write_artifacts(path, &dataset, &options, &censuses)?;
            seed(&store, "Arrived During Render");
            Ok(())
        })
        .expect_err("source fence must refuse stale publication");
    assert!(error.to_string().contains("stale or foreign"));
    let root = store.out_dir().join("publication");
    assert_eq!(
        current_workbook(&root).expect("current publication"),
        published
    );
    verify_published(&published).expect("previous publication remains valid");
    assert_no_temporaries(&root);
}

#[test]
fn an_export_job_keeps_its_input_and_changed_evidence_requires_a_new_export() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    seed(&store, "First School");
    let first = ExportDataset::for_job(&store, "export:first-input").expect("freeze first job");
    store
        .journal_done("unrelated-work", "attempt", &true)
        .expect("unrelated journal write");
    let replay =
        ExportDataset::for_job(&store, "export:first-input").expect("replay unchanged input");
    assert_eq!(first.lineage, replay.lineage);
    seed(&store, "Later School");
    let error = ExportDataset::for_job(&store, "export:first-input")
        .err()
        .expect("stale job refused");
    assert!(error.to_string().contains("stale or foreign"));
    let next =
        ExportDataset::for_job(&store, "export:changed-input").expect("capture new logical input");
    assert_ne!(
        next.lineage.input_generation,
        first.lineage.input_generation
    );
    assert_eq!(first.schools.len(), 1);
    assert_eq!(next.schools.len(), 2);
    let archive = store.out_dir().join("export-inputs").join(format!(
        "{}.json",
        census_domain::model::serialized_digest(&"export:first-input").expect("job digest")
    ));
    let retained = ExportDataset::reopen_frozen(&archive).expect("old input still retained");
    assert_eq!(retained.lineage, first.lineage);
}
