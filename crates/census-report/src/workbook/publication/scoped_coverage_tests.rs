use crate::export::ExportDataset;
use crate::report::{
    build_census, coverage_report, coverage_report_for_derivation, Derivation, Scope,
};
use crate::workbook::{Options, PerformanceProjection};
use calamine::{open_workbook, Data, Reader, Xlsx};
use census_domain::model::{serialized_bytes, GradYear, SchoolYear};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use census_store::Store;
use std::path::Path;

#[path = "../../../../../fixtures/publication-scoped-coverage.rs"]
mod fixture;
mod scoped_coverage_readback;

use scoped_coverage_readback::{assert_cells, tamper_coverage};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn options(scope: Scope, jurisdictions: Vec<UsJurisdiction>) -> TestResult<Options> {
    Ok(Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope,
        jurisdictions,
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
    })
}

#[test]
fn restricted_coverage_uses_shared_population_and_retains_whole_frozen_evidence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let options = options(Scope::AllSources, vec![UsJurisdiction::Wisconsin])?;
    let path = crate::workbook::build(&store, &options)?;
    let archived = path
        .parent()
        .ok_or("publication directory")?
        .join("frozen-input.json");
    let dataset = ExportDataset::reopen_frozen(&archived)?;
    let before = serialized_bytes(&dataset)?;
    let global_before = serde_json::to_vec(&coverage_report(&dataset, Some(2027))?)?;
    let derivation = Derivation::of(&dataset, options.scope, Some(2027), &options.jurisdictions);
    let coverage = coverage_report_for_derivation(&derivation)?;
    let census = build_census(&derivation, &store.out_dir());
    check!(eq; coverage.read.schools, 2);
    check!(eq; coverage.read.athletes, 3);
    check!(eq; coverage.read.coaches, 2);
    check!(eq; coverage.read.meets, 1);
    check!(eq; coverage.read.performances, 3);
    check!(eq; coverage.event_catalog.included, 3);
    let selected_event_ids: std::collections::HashSet<_> = derivation
        .performances()
        .iter()
        .map(|row| row.event.as_str())
        .collect();
    check!(eq; selected_event_ids.len(), 2);
    check!(eq; coverage.read.athletes, census.totals.athletes);
    check!(eq; coverage.read.coaches, census.totals.coaches);
    check!(eq; coverage.read.meets, census.meets.total);
    check!(eq; coverage.source_population.registered, Some(7));
    check!(eq; coverage.contact.pending, 1);
    check!(eq; coverage.contact.blocked, 0);
    check!(eq; coverage.raw_observations, 6);
    check!(eq; coverage.scope_filtered, 5);
    check!(eq; coverage.off_cohort_athletes, 1);
    check!(eq; coverage.jurisdictions.len(), 2);
    let unplaced = coverage
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == JurisdictionBucket::Unplaced)
        .ok_or("unplaced bucket")?;
    check!(eq; unplaced.athletes, 1);
    check!(eq; unplaced.source_population.registered, None);
    check!(eq; serde_json::to_value(unplaced.state)?, serde_json::json!("unresearched"));
    check!(coverage
        .notes
        .iter()
        .any(|note| note.contains("UNPLACED") && note.contains("does not certify")));
    check!(coverage.notes.iter().any(|note| note
        .contains("retained outside requested jurisdictions MN:")
        && note.contains("contact_blocked=1")
        && note.contains("registered=9")));
    check!(coverage.notes.iter().any(|note| note
        .contains("retained raw gap outside requested jurisdictions")
        && note.contains("jurisdiction=MN")));
    check!(coverage
        .notes
        .iter()
        .any(|note| note.contains("class=missing_graduation_evidence")
            && note.contains("before evidence/cohort selection")));
    check!(eq; dataset.schools.len(), 3);
    check!(eq; dataset.athletes.len(), 6);
    check!(eq; dataset.performances.len(), 4);
    check!(eq; dataset.coverage_evidence.rows.len(), 2);
    check!(eq; dataset.coverage_evidence.coach_lanes.len(), 2);
    coverage.reconcile()?;
    crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    assert_cells(&path, 3, 2, 1, 3)?;
    assert_historical_parent(&derivation)?;
    check!(eq; serialized_bytes(&dataset)?, before);
    check!(eq; serde_json::to_vec(&coverage_report(&dataset, Some(2027))?)?, global_before);
    let target = directory.path().join("tampered.xlsx");
    tamper_coverage(&path, &target)?;
    let error = crate::workbook::verify::verify_frozen(&target, &dataset, &options)
        .err()
        .ok_or("tampered scoped Coverage accepted")?;
    check!(error.to_string().contains("Coverage"));
    Ok(())
}

fn assert_historical_parent(derivation: &Derivation<'_>) -> TestResult {
    let projection = PerformanceProjection::of(derivation);
    let row = derivation
        .performances()
        .iter()
        .find(|row| row.source_key == "wi-historical")
        .ok_or("in-scope historical result")?;
    let published = serde_json::to_value(projection.row(row))?;
    check!(eq; published.get("school").and_then(serde_json::Value::as_str), Some("Historical Minnesota"));
    check!(eq; published.get("meet").and_then(serde_json::Value::as_str), Some("Historical Minnesota Meet"));
    check!(eq; published.get("state").and_then(serde_json::Value::as_str), Some("MN"));
    Ok(())
}

#[test]
fn empty_jurisdictions_keep_national49_and_legacy_global_semantics() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let options = options(Scope::AllSources, Vec::new())?;
    let path = crate::workbook::build(&store, &options)?;
    let archived = path
        .parent()
        .ok_or("publication directory")?
        .join("frozen-input.json");
    let dataset = ExportDataset::reopen_frozen(&archived)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027), &[]);
    let report = coverage_report_for_derivation(&derivation)?;
    check!(eq; report.jurisdictions.iter().filter(|row| row.jurisdiction.jurisdiction().is_some()).count(), 49);
    check!(eq; report.jurisdictions.len(), 50);
    check!(eq; report.read.athletes, 4);
    check!(eq; report.read.performances, 4);
    check!(eq; report.source_population.registered, None);
    check!(eq; report.raw_observations, 6);
    check!(eq; report.scope_filtered, 6);
    check!(eq; report.contact.blocked, 1);
    check!(eq; report.contact.never_attempted, 47);
    check!(report.notes.iter().any(|note| note.contains("known outside requested jurisdictions [schools=0 athletes=0 coaches=0 meets=0 performances=0]")));
    let global = coverage_report(&dataset, Some(2027))?;
    check!(eq; global.read.athletes, 5);
    check!(eq; global.source_population.registered, Some(16));
    check!(eq; global.jurisdictions.len(), 50);
    report.reconcile()?;
    crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    assert_cells(&path, 4, 3, 2, 4)?;
    Ok(())
}

#[test]
fn core_scope_excludes_noncore_population_without_erasing_raw_gap_counts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let options = options(Scope::Core, vec![UsJurisdiction::Wisconsin])?;
    let path = crate::workbook::build(&store, &options)?;
    let dataset = ExportDataset::reopen_frozen(
        &path
            .parent()
            .ok_or("publication directory")?
            .join("frozen-input.json"),
    )?;
    let derivation = Derivation::of(
        &dataset,
        Scope::Core,
        Some(GradYear::CO2027.get()),
        &options.jurisdictions,
    );
    let coverage = coverage_report_for_derivation(&derivation)?;
    check!(eq; coverage.read.athletes, 2);
    check!(eq; coverage.read.performances, 2);
    check!(eq; coverage.raw_observations, 6);
    check!(eq; coverage.scope_filtered, 4);
    check!(coverage.notes.iter().any(|note| note.contains(
        "evidence/cohort excluded [schools=0 athletes=3 coaches=0 meets=0 performances=1]"
    )));
    assert_historical_parent(&derivation)?;
    crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    assert_cells(&path, 2, 2, 1, 2)?;
    Ok(())
}

#[test]
fn default_scope_retains_noncontinental_evidence_and_outside_gap_notes() -> TestResult {
    use census_domain::model::{
        CanonicalAthlete, CanonicalSchool, Gender, PublishedGraduation, SourceIdentity,
        SourceNamespace, SourceRef,
    };
    use census_store::Table;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture::seed(&store)?;
    let (school, id) = CanonicalSchool::new(
        UsJurisdiction::Alaska,
        "Retained Alaska",
        "retained alaska",
        None,
    );
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &id,
        "Retained Alaska Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "alaska-retained"),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::id("asaaa_results"),
    });
    store.append(Table::Athletes, &athlete)?;
    let meet = fixture::meet(&store, UsJurisdiction::Alaska, "Alaska Meet")?;
    fixture::result(
        &store,
        &athlete,
        &school,
        &meet,
        "ak-result",
        "wiaa_results",
    )?;
    let options = options(Scope::AllSources, Vec::new())?;
    let path = crate::workbook::build(&store, &options)?;
    let dataset = ExportDataset::reopen_frozen(
        &path
            .parent()
            .ok_or("publication directory")?
            .join("frozen-input.json"),
    )?;
    let report = coverage_report_for_derivation(&Derivation::of(
        &dataset,
        Scope::AllSources,
        Some(2027),
        &[],
    ))?;
    check!(eq; report.jurisdictions.len(), 50);
    check!(eq; report.read.athletes, 4);
    check!(eq; report.raw_observations, 7);
    check!(eq; report.scope_filtered, 6);
    check!(eq; dataset.athletes.len(), 7);
    check!(eq; dataset.schools.len(), 4);
    check!(report.notes.iter().any(|note| note.contains("known outside requested jurisdictions [schools=1 athletes=1 coaches=0 meets=1 performances=1]")));
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("raw PR support")
            && note.contains("AK/HI")
            && note.contains("unmeasured")));
    check!(!report
        .notes
        .iter()
        .any(|note| note.contains("jurisdiction=AK") && note.contains("class=missing_pr_support")));
    check!(eq; dataset.performances.len(), 5);
    check!(!report
        .jurisdictions
        .iter()
        .any(|row| row.jurisdiction.code() == "AK"));
    crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    Ok(())
}
