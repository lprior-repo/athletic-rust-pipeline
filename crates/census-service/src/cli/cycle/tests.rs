use super::{offline_stages, run_cycle, PublicationPreparation};
use crate::cli::{Cli, Command};
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear,
    IdentityStatus, PublishedGraduation, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_report::{report::Scope, workbook};
use census_service::restate_services::{
    workbook_request_key, ExportGeneration, PublicationCapabilities, WorkbookRequest,
};
use census_store::{Store, Table};
use clap::Parser;

mod live;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn options(root: &std::path::Path) -> TestResult<workbook::Options> {
    Ok(workbook::Options {
        grad_year: Some(2027),
        out: Some(root.join("publication")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    })
}

fn unaccepted_athlete(store: &Store) -> TestResult<CanonicalAthlete> {
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Fixture", "fixture", None);
    let source = SourceRef::new("fixture", Some("https://example.test/results".into()));
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Fixture Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    )?;
    athlete
        .evidence
        .push(Evidence::parsed(source.clone(), "2026-10-12"));
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source,
    });
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    Ok(athlete)
}

fn check_managed_only(store: &Store, root: &std::path::Path) -> TestResult {
    let published = workbook::publication::current_workbook(&root.join("publication"))?;
    workbook::publication::verify_published(&published)?;
    let directory = published
        .parent()
        .ok_or("managed generation has no directory")?;
    for name in [
        "athletes.csv",
        "best_marks.csv",
        "schools.csv",
        "coach_contacts.csv",
        "comparison_classes.csv",
        "provenance.csv",
        "coverage.csv",
        "identity_aliases.csv",
    ] {
        check!(
            directory.join(name).is_file(),
            "managed artifact missing: {name}"
        );
    }
    for name in [
        "athletes.jsonl",
        "schools.jsonl",
        "census-all-sources.json",
        "census-core.json",
        "census-by-state.csv",
        "census-by-state-core.csv",
        "best-results-co2027.csv",
    ] {
        check!(
            !store.out_dir().join(name).try_exists()?,
            "flat artifact was written: {name}"
        );
    }
    Ok(())
}

#[test]
fn gathered_inputs_are_identity_prepared_before_managed_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let athlete = unaccepted_athlete(&store)?;
    check!(eq; store.athlete_identity_projection()?.status(athlete.id.as_str())?, IdentityStatus::Unverified);
    offline_stages(
        &store,
        &options(directory.path())?,
        PublicationPreparation::Gathered {
            observed_on: "2026-10-12".into(),
        },
    )?;
    check!(eq; store.athlete_identity_projection()?.status(athlete.id.as_str())?, IdentityStatus::Verified);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 1);
    let published = workbook::publication::current_workbook(&directory.path().join("publication"))?;
    let csv_path = published
        .parent()
        .ok_or("generation directory missing")?
        .join("athletes.csv");
    let rows = csv::Reader::from_path(csv_path)?
        .records()
        .collect::<Result<Vec<_>, _>>()?;
    check!(rows
        .iter()
        .any(|row| row.iter().any(|field| field == athlete.id.as_str())));
    check_managed_only(&store, directory.path())?;
    Ok(())
}

#[test]
fn retained_publication_preserves_prepared_identity_and_derived_state() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let athlete = unaccepted_athlete(&store)?;
    let options = options(directory.path())?;
    census_reconcile::index::derive(&store, "fixture", "2026-10-12", options.school_year)?;
    let generation = store.derived_generation();
    let identities = store.walk_table(Table::AthleteIdentityDecisions)?;
    let snapshots = store.walk_table(Table::Snapshots)?;
    let coverage = store.walk_table(Table::Coverage)?;
    let digest = store.snapshot().tables_digest(&Table::ALL)?;
    offline_stages(&store, &options, PublicationPreparation::Retained)?;
    check!(eq; store.derived_generation(), generation);
    check!(eq; store.walk_table(Table::AthleteIdentityDecisions)?, identities);
    check!(eq; store.walk_table(Table::Snapshots)?, snapshots);
    check!(eq; store.walk_table(Table::Coverage)?, coverage);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, digest);
    check!(eq; store.athlete_identity_projection()?.status(athlete.id.as_str())?, IdentityStatus::Verified);
    check_managed_only(&store, directory.path())?;
    Ok(())
}
