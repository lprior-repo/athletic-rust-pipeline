use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CensusRun, CoachContactClaim,
    CoachContactProgram, CoachRole, CoachTenure, CoachTenureEvidence, Evidence, Gender, GradYear,
    PublishedGraduation, RunManifest, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    Sport,
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
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
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
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
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
    let published = crate::workbook::build(
        &store,
        &Options {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: census_domain::model::SchoolYear::new(2026)
                .ok_or("invalid fixture season")?,
        },
    )?;
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
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
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

#[test]
fn contact_season_follows_the_requested_year_not_the_export_date() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed_contact_age(
        &store,
        "Twenty Five High School",
        2025,
        "coach25@example.test",
    )?;
    seed_contact_age(
        &store,
        "Twenty Six High School",
        2026,
        "coach26@example.test",
    )?;
    let older = build_for_season(&store, &directory.path().join("season-2025"), 2025)?;
    let newer = build_for_season(&store, &directory.path().join("season-2026"), 2026)?;
    check!(eq; coach_emails(&older)?, ["coach25@example.test"]);
    check!(eq; coach_emails(&newer)?, ["coach26@example.test"]);
    Ok(())
}

fn seed_contact_age(store: &Store, name: &str, season: i16, mailbox: &str) -> TestResult {
    let (mut school, school_id) =
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
        &school_id,
        "Cy Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), name),
    );
    athlete.sports = vec![Sport::CrossCountry];
    let claim = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "wiaa_results",
            Some("https://example.test/results/2027".to_string()),
        ),
    };
    athlete
        .evidence
        .push(Evidence::parsed(claim.source.clone(), "2026-06-01"));
    athlete.published_graduations.push(claim);
    athlete.evidence.push(evidence.clone());
    store.append(Table::Athletes, &athlete)?;
    let mut coach = CanonicalCoach::new(
        &school_id,
        "Sam Coach",
        Some(Sport::CrossCountry),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(mailbox.to_string());
    coach.evidence.push(evidence);
    coach.tenure_evidence.push(CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(season).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new(
            "coach_contacts",
            Some("https://example.test/coaches".to_string()),
        ),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: format!("Synthetic appointment for the {season} season"),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: school_id,
            role: CoachRole::HeadCoach,
            program: CoachContactProgram::Team {
                sport: Sport::CrossCountry,
                gender: Gender::Girls,
            },
            mailbox: Some(mailbox.to_string()),
        }),
    });
    store.append(Table::Coaches, &coach)?;
    Ok(())
}

fn build_for_season(store: &Store, out: &std::path::Path, season: i16) -> TestResult<PathBuf> {
    Ok(crate::workbook::build(
        store,
        &Options {
            grad_year: Some(2027),
            out: Some(out.to_path_buf()),
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(season).ok_or("invalid fixture season")?,
        },
    )?)
}

fn coach_emails(workbook: &std::path::Path) -> TestResult<Vec<String>> {
    let path = workbook
        .parent()
        .ok_or("missing generation directory")?
        .join("recruiting.csv");
    let mut reader = csv::Reader::from_path(&path)?;
    let header = reader.headers()?.clone();
    let column = header
        .iter()
        .position(|field| field == "head_xc_coach_email")
        .ok_or("missing published column head_xc_coach_email")?;
    let mut emails = Vec::new();
    for row in reader.records() {
        let row = row?;
        if let Some(value) = row.get(column).filter(|value| !value.is_empty()) {
            emails.push(value.to_string());
        }
    }
    Ok(emails)
}

fn bind_census_run(store: &Store) -> TestResult {
    let season = SchoolYear::new(2026).ok_or("2026 is a school year")?;
    store.bind_run(&RunManifest {
        store_identity: crate::export::store_identity(store)?,
        run: CensusRun::new(season, 1).ok_or("a run revision starts at one")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
    })?;
    Ok(())
}

#[test]
fn a_seal_is_bound_to_the_run_and_cohort_the_store_declares() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let store = Store::open(&root)?;
    seed(&store, "Bound High School")?;
    bind_census_run(&store)?;
    let workbook = build_for_season(&store, &directory.path().join("publication"), 2026)?;
    let dataset = ExportDataset::load(&store)?;
    check!(eq;
        dataset.lineage.run.as_ref().map(|run| run.revision()),
        Some(1),
        "the export names the run the store was bound to"
    );
    check!(eq; dataset.lineage.cohort, Some(GradYear::CO2027));

    verify_for_seal(
        &workbook,
        &dataset,
        crate::report::Scope::AllSources,
        GradYear::CO2027,
    )?;

    let later = GradYear::new(2028).ok_or("2028 is a cohort year")?;
    match verify_for_seal(&workbook, &dataset, crate::report::Scope::AllSources, later) {
        Err(error) => check!(
            error
                .to_string()
                .contains("Class-of-2027 cohort is the only cohort"),
            "the refusal names the only sealable cohort: {error}"
        ),
        Ok(_) => return Err("a 2028 seal of a store bound to the 2027 census is refused".into()),
    }
    Ok(())
}

#[test]
fn a_publication_of_another_stores_run_cannot_seal_this_census() -> TestResult {
    let directory = tempfile::tempdir()?;
    let bound = Store::open(directory.path().join("bound"))?;
    seed(&bound, "Bound High School")?;
    bind_census_run(&bound)?;
    let workbook = build_for_season(&bound, &directory.path().join("publication"), 2026)?;

    let loose = Store::open(directory.path().join("loose"))?;
    seed(&loose, "Loose High School")?;
    let dataset = ExportDataset::load(&loose)?;
    check!(
        dataset.lineage.run.is_none(),
        "a store no national run admitted carries no run"
    );

    match verify_for_seal(
        &workbook,
        &dataset,
        crate::report::Scope::AllSources,
        GradYear::CO2027,
    ) {
        Err(error) => check!(
            error.to_string().contains("different census run"),
            "the refusal names the run disagreement: {error}"
        ),
        Ok(_) => return Err("a publication of another store's run cannot seal this one".into()),
    }
    Ok(())
}

#[test]
fn a_publication_of_another_cohort_never_seals_this_census() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let store = Store::open(&root)?;
    seed(&store, "Cohort High School")?;
    bind_census_run(&store)?;
    let workbook = crate::workbook::build(
        &store,
        &Options {
            grad_year: Some(2028),
            out: Some(directory.path().join("publication-2028")),
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("2026 is a school year")?,
        },
    )?;
    let dataset = ExportDataset::load(&store)?;
    let later = GradYear::new(2028).ok_or("2028 is a cohort year")?;

    match verify_for_seal(&workbook, &dataset, crate::report::Scope::AllSources, later) {
        Err(error) => check!(
            error
                .to_string()
                .contains("Class-of-2027 cohort is the only cohort"),
            "a bound census refuses the other cohort's publication: {error}"
        ),
        Ok(_) => {
            return Err("a publication restricted to 2028 cannot certify the 2027 census".into())
        }
    }
    match verify_for_seal(
        &workbook,
        &dataset,
        crate::report::Scope::AllSources,
        GradYear::CO2027,
    ) {
        Err(_) => {}
        Ok(_) => return Err("a 2028 publication is not a Class-of-2027 seal".into()),
    }
    Ok(())
}

#[test]
fn a_publication_assessed_in_another_school_year_cannot_seal_the_run() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let store = Store::open(&root)?;
    seed(&store, "Season High School")?;
    bind_census_run(&store)?;
    let workbook = crate::workbook::build(
        &store,
        &Options {
            grad_year: Some(2027),
            out: Some(directory.path().join("publication-2027")),
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2027).ok_or("2027 is a school year")?,
        },
    )?;
    let dataset = ExportDataset::load(&store)?;
    match verify_for_seal(
        &workbook,
        &dataset,
        crate::report::Scope::AllSources,
        GradYear::CO2027,
    ) {
        Err(error) => check!(
            error.to_string().contains("school year"),
            "the refusal names the run's own school year: {error}"
        ),
        Ok(_) => {
            return Err("a publication assessed in another school year cannot seal the run".into())
        }
    }
    Ok(())
}

#[test]
fn a_store_bound_to_another_cohort_cannot_receive_the_census_seal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed(&store, "Foreign Cohort High School")?;
    store.bind_run(&RunManifest {
        store_identity: crate::export::store_identity(&store)?,
        run: CensusRun::new(SchoolYear::new(2026).ok_or("2026 is a school year")?, 1)
            .ok_or("a run revision starts at one")?,
        cohort: GradYear::new(2028).ok_or("2028 is a cohort year")?,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
    })?;
    let workbook = crate::workbook::build(
        &store,
        &Options {
            grad_year: Some(2028),
            out: Some(directory.path().join("publication-2028")),
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("2026 is a school year")?,
        },
    )?;
    let dataset = ExportDataset::load(&store)?;
    check!(eq; dataset.lineage.cohort, Some(GradYear::new(2028).ok_or("2028 is a cohort year")?));
    let later = GradYear::new(2028).ok_or("2028 is a cohort year")?;
    match verify_for_seal(&workbook, &dataset, crate::report::Scope::AllSources, later) {
        Err(error) => check!(
            error
                .to_string()
                .contains("Class-of-2027 cohort is the only cohort"),
            "the seal boundary names the only cohort a seal may certify: {error}"
        ),
        Ok(_) => {
            return Err("a store bound to the 2028 cohort cannot receive the census seal".into())
        }
    }
    Ok(())
}

#[test]
fn a_publication_of_another_runs_revision_cannot_seal_this_census() -> TestResult {
    let directory = tempfile::tempdir()?;
    let publishing = Store::open(directory.path().join("publishing"))?;
    seed(&publishing, "Publishing High School")?;
    bind_census_run(&publishing)?;
    let workbook = build_for_season(&publishing, &directory.path().join("publication"), 2026)?;

    let measured = Store::open(directory.path().join("measured"))?;
    seed(&measured, "Measured High School")?;
    measured.bind_run(&RunManifest {
        store_identity: crate::export::store_identity(&measured)?,
        run: CensusRun::new(SchoolYear::new(2026).ok_or("2026 is a school year")?, 2)
            .ok_or("a run revision starts at one")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
    })?;
    let dataset = ExportDataset::load(&measured)?;
    check!(eq;
        dataset.lineage.run.as_ref().map(|run| run.revision()),
        Some(2),
        "the measured store carries its own run revision"
    );

    match verify_for_seal(
        &workbook,
        &dataset,
        crate::report::Scope::AllSources,
        GradYear::CO2027,
    ) {
        Err(error) => check!(
            error.to_string().contains("different census run"),
            "the refusal names the run disagreement: {error}"
        ),
        Ok(_) => {
            return Err("a publication of another run's revision cannot seal this census".into())
        }
    }
    Ok(())
}
