use super::{seal, SealRequest, SealWorkflowError};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_report::export::{
    decoded_generations, peak_resident_generations, resident_generations, ExportDataset,
};
use census_report::report::Scope;
use census_store::{Store, Table};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn options() -> TestResult<census_report::workbook::Options> {
    Ok(census_report::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    })
}

fn populated_store(path: &Path) -> TestResult<Store> {
    let store = Store::open(path)?;
    let (mut school, id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Test School",
        "test school",
        None,
    );
    let evidence = Evidence::parsed(
        SourceRef::new(
            "milesplit",
            Some("https://www.milesplit.com/athletes/1234".into()),
        ),
        "2026-06-01",
    );
    school.evidence.push(evidence.clone());
    let mut athlete = CanonicalAthlete::new(
        &id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1234"),
    );
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: evidence.source.clone(),
        });
    athlete.evidence.push(evidence);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    Ok(store)
}

fn request(path: &Path, write: bool) -> SealRequest {
    SealRequest {
        grad_year: 2027,
        scope: Scope::AllSources,
        workbook: Some(path.to_path_buf()),
        write,
        journal: None,
        source_failures: None,
        run: None,
    }
}

#[test]
fn a_seal_holds_one_resident_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let path = census_report::workbook::build(&store, &options()?)?;
    let baseline = resident_generations();
    let peak_before = peak_resident_generations();
    let decoded_baseline = decoded_generations();
    check!(
        eq;
        resident_generations(),
        baseline,
        "the build released its decoded generation"
    );
    check!(
        peak_before <= baseline.saturating_add(1),
        "the fixture build itself holds at most one complete generation"
    );
    let outcome = seal(&store, &request(&path, false))?;
    check!(
        eq;
        decoded_generations(),
        decoded_baseline.saturating_add(1),
        "the seal decodes exactly one complete generation"
    );
    check!(
        eq;
        peak_resident_generations(),
        peak_before,
        "a second complete generation was never resident during the seal"
    );
    check!(
        eq;
        resident_generations(),
        baseline,
        "the shared generation is released when the seal finishes"
    );
    check!(
        eq;
        census_report::workbook::publication::current_workbook(
            &store.out_dir().join("publication")
        )?,
        path,
        "the seal certifies the frozen bundle the publication carries"
    );
    check!(
        outcome.state.sealed().is_none() && outcome.wrote.is_none(),
        "an unbound store refuses the seal without writing a certificate"
    );
    Ok(())
}

#[test]
fn a_seal_residency_probe_detects_a_second_complete_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let baseline = resident_generations();
    let first = ExportDataset::load(&store)?;
    check!(
        eq;
        resident_generations(),
        baseline.saturating_add(1),
        "one complete generation is visible to the probe"
    );
    let second = ExportDataset::load(&store)?;
    check!(
        eq;
        resident_generations(),
        baseline.saturating_add(2),
        "a second complete generation would be visible to the probe"
    );
    check!(
        peak_resident_generations() >= baseline.saturating_add(2),
        "the resident peak covers both complete generations"
    );
    drop(second);
    check!(
        eq;
        resident_generations(),
        baseline.saturating_add(1),
        "releasing one generation is visible to the probe"
    );
    drop(first);
    check!(
        eq;
        resident_generations(),
        baseline,
        "releasing the last generation restores the baseline"
    );
    Ok(())
}

#[test]
fn a_refused_seal_retains_the_prior_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let path = census_report::workbook::build(&store, &options()?)?;
    let publication_root = store.out_dir().join("publication");
    let recorded = store.out_dir().join("seal.json");
    std::fs::write(&recorded, b"prior certificate bytes")?;
    let before = std::fs::read(&recorded)?;
    let (later, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Later School",
        "later school",
        None,
    );
    store.append(Table::Schools, &later)?;
    let decoded_baseline = decoded_generations();
    let error = match seal(&store, &request(&path, true)) {
        Err(error) => error,
        Ok(_) => return Err("a stale publication sealed".into()),
    };
    check!(
        matches!(&error, SealWorkflowError::Workbook(_)),
        "the refusal comes from the publication preflight: {error}"
    );
    check!(
        eq;
        decoded_generations(),
        decoded_baseline,
        "the stale store was refused before any table decode"
    );
    check!(
        eq;
        std::fs::read(&recorded)?,
        before,
        "a refused seal leaves the prior certificate untouched"
    );
    check!(
        eq;
        census_report::workbook::publication::current_workbook(&publication_root)?,
        path,
        "a refused seal leaves the published generation in place"
    );
    Ok(())
}
