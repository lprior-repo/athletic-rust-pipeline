use super::{full_options, TestResult};
use crate::census::seal::{seal, JournalCounts, SealRequest};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, CoverageRow};
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report::{build_census, coverage_report_for_derivation, Derivation, Scope};
use census_store::{Store, Table};

#[path = "../../../../../../fixtures/publication-scoped-coverage.rs"]
mod fixture;

#[test]
fn seal_counts_reconcile_with_restricted_workbook_census_and_coverage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let options = census_report::workbook::Options {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        ..full_options()?
    };
    let path = census_report::workbook::build(&store, &options)?;
    let archive = path
        .parent()
        .ok_or("publication directory")?
        .join("frozen-input.json");
    let bytes = std::fs::read(&archive)?;
    let dataset = ExportDataset::reopen_frozen(&archive)?;
    let derivation = Derivation::of(
        &dataset,
        Scope::AllSources,
        Some(2027),
        &options.jurisdictions,
    )?;
    let coverage = coverage_report_for_derivation(&derivation)?;
    let census = build_census(&derivation, &store.out_dir());
    check!(eq; coverage.read.athletes, census.totals.athletes);
    check!(eq; coverage.read.coaches, census.totals.coaches);
    check!(eq; coverage.read.meets, census.meets.total);
    let [schools, athletes, coaches, meets, performances] = [
        coverage.read.schools,
        coverage.read.athletes,
        coverage.read.coaches,
        coverage.read.meets,
        coverage.read.performances,
    ];
    let retained = store.stats()?.observations;
    drop(derivation);
    drop(dataset);
    let request = SealRequest {
        grad_year: 2027,
        scope: options.scope,
        jurisdictions: options.jurisdictions.clone(),
        workbook: Some(path),
        write: false,
        journal: Some(JournalCounts {
            jurisdiction_sweeps: Some(0),
            source_objects: Some(0),
            silent_sources: Vec::new(),
        }),
        source_failures: Some(0),
        run: None,
    };
    let outcome = seal(&store, &request)?;
    let counts = &outcome.evidence.counts;
    check!(eq; counts.jurisdiction_buckets, 2);
    check!(eq; counts.schools, u64::try_from(schools)?);
    check!(eq; counts.athletes, u64::try_from(athletes)?);
    check!(eq; counts.class_of_2027, counts.athletes);
    check!(eq; counts.coaches, u64::try_from(coaches)?);
    check!(eq; counts.meets, u64::try_from(meets)?);
    check!(eq; counts.cohort_performances, u64::try_from(performances)?);
    check!(eq; outcome.evidence.retained.calculations, counts.cohort_performances);
    check!(eq; outcome.evidence.retained.observations, retained);
    check!(
        outcome.state.sealed().is_none(),
        "an unbound run does not certify unknown jurisdiction inclusion"
    );
    check!(eq; std::fs::read(&archive)?, bytes);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 3);
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 6);
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?.len(), 2);
    Ok(())
}

#[test]
fn scoped_core_seal_counts_keep_retained_store_observations_global() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let options = census_report::workbook::Options {
        scope: Scope::Core,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        ..full_options()?
    };
    let path = census_report::workbook::build(&store, &options)?;
    let retained = store.stats()?.observations;
    let request = SealRequest {
        grad_year: 2027,
        scope: Scope::Core,
        jurisdictions: options.jurisdictions,
        workbook: Some(path),
        write: false,
        journal: None,
        source_failures: None,
        run: None,
    };
    let evidence = seal(&store, &request)?.evidence;
    check!(eq; evidence.counts.athletes, 2);
    check!(eq; evidence.counts.cohort_performances, 2);
    check!(eq; evidence.counts.schools, 2);
    check!(eq; evidence.counts.meets, 1);
    check!(eq; evidence.counts.coaches, 2);
    check!(eq; evidence.counts.jurisdiction_buckets, 2);
    check!(eq; evidence.retained.observations, retained);
    Ok(())
}

#[test]
fn outside_only_raw_conflict_remains_in_retained_seal_findings() -> TestResult {
    use census_domain::model::{GradYear, PublishedGraduation, SourceRef};
    use census_report::report::{coverage_report, GapClass};
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let mut outside = store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|athlete| athlete.canonical_name == "Minnesota Runner")
        .ok_or("outside athlete")?;
    outside.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::new(2028).ok_or("fixture conflicting year")?,
        source: SourceRef::id("mshsl_results"),
    });
    store.append(Table::Athletes, &outside)?;
    let options = census_report::workbook::Options {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        ..full_options()?
    };
    let path = census_report::workbook::build(&store, &options)?;
    let archive = path
        .parent()
        .ok_or("publication directory")?
        .join("frozen-input.json");
    let dataset = ExportDataset::reopen_frozen(&archive)?;
    let derivation = Derivation::of(
        &dataset,
        Scope::AllSources,
        Some(2027),
        &options.jurisdictions,
    )?;
    let scoped = coverage_report_for_derivation(&derivation)?;
    check!(!scoped
        .gaps
        .iter()
        .any(|gap| gap.class == GapClass::ConflictingIdentity));
    let global = coverage_report(&dataset, Some(2027))?;
    check!(global.gaps.iter().any(|gap| gap.jurisdiction.code() == "MN"
        && gap.class == GapClass::ConflictingIdentity
        && gap.count == 1));
    drop(derivation);
    drop(dataset);
    let request = SealRequest {
        grad_year: 2027,
        scope: Scope::AllSources,
        jurisdictions: options.jurisdictions,
        workbook: Some(path),
        write: false,
        journal: None,
        source_failures: None,
        run: None,
    };
    let evidence = seal(&store, &request)?.evidence;
    check!(evidence
        .retained
        .gaps
        .iter()
        .any(|gap| gap.class == "conflicting_identity" && gap.count == 1));
    check!(eq; evidence.counts.athletes, 3);
    check!(eq; evidence.counts.cohort_performances, 3);
    Ok(())
}
