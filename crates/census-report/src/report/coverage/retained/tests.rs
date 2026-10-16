use super::super::{coverage_report, coverage_report_for_derivation, pr_support, scan::Scanned};
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::report::{CoverageReport, Derivation, Scope};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

pub(crate) mod correction;
mod fixture;

use fixture::selected_dataset;

fn selected_report(
    dataset: &crate::export::ExportDataset,
    jurisdictions: &[UsJurisdiction],
) -> TestResult<CoverageReport> {
    let derivation = Derivation::for_publication(dataset, Scope::AllSources, None, jurisdictions)?;
    Ok(coverage_report_for_derivation(&derivation)?)
}

fn require_population(
    report: &CoverageReport,
    retained: usize,
    published: usize,
    outside: usize,
) -> TestResult {
    check!(eq; report.raw_observations, 3);
    check!(eq; report.read.athletes, published);
    check!(eq; report.published_totals().athletes, published);
    check!(report.notes.iter().any(|note| note == &format!(
        "selected accepted people reconciliation: retained={retained} = publication={published} + known outside requested jurisdictions={outside} + current affiliation/evidence/cohort excluded=0",
    )));
    check!(report.notes.iter().any(|note| note ==
        "selected identity candidate audit: retained_candidate_records=3 accepted_candidate_records=2 withheld_or_unknown_candidate_records=1 absent_candidate_records=0; candidate records are not distinct accepted people"));
    report.reconcile()?;
    Ok(())
}

#[test]
fn selected_births_count_two_accepted_people_and_audit_unknown_candidate_separately() -> TestResult
{
    let fixture = source_births()?;
    let dataset = selected_dataset(&fixture.store)?;
    let unchanged = serde_json::to_vec(&dataset)?;
    let scanned = Scanned::of(&dataset)?;
    check!(eq; scanned.athletes.len(), 2);
    check!(scanned.aliases.is_empty());
    let [first, second, unknown] = &fixture.athletes;
    check!(scanned
        .athletes
        .iter()
        .all(|athlete| athlete.id != unknown.id));
    let report = selected_report(&dataset, &[])?;
    require_population(&report, 2, 2, 0)?;
    let mut tally = report.clone();
    pr_support::tally_pr_support(
        &mut tally,
        &dataset,
        None,
        &HashSet::from([first.id.as_str(), second.id.as_str(), unknown.id.as_str()]),
    )?;
    check!(eq; tally.jurisdictions.iter().map(|row| row.with_pr_support).sum::<usize>(), 2);
    check!(eq; coverage_report(&dataset, None)?.read.athletes, 2);
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    Ok(())
}

#[test]
fn retained_merge_reconciles_one_person_in_publication_or_current_outside_population() -> TestResult
{
    let fixture = source_births()?;
    merge(&fixture)?;
    let dataset = selected_dataset(&fixture.store)?;
    let unchanged = serde_json::to_vec(&dataset)?;
    let [first, second, unknown] = &fixture.athletes;
    let scanned = Scanned::of(&dataset)?;
    check!(eq; scanned.athletes.len(), 1);
    check!(eq; scanned.athletes.first().ok_or("accepted representative")?.id, first.id);
    check!(eq; scanned.aliases.get(second.id.as_str()).map(String::as_str), Some(first.id.as_str()));
    let report = selected_report(&dataset, &[UsJurisdiction::Michigan])?;
    require_population(&report, 1, 1, 0)?;
    let mut tally = report.clone();
    pr_support::tally_pr_support(
        &mut tally,
        &dataset,
        None,
        &HashSet::from([first.id.as_str(), second.id.as_str(), unknown.id.as_str()]),
    )?;
    check!(eq; tally.jurisdictions.iter().map(|row| row.with_pr_support).sum::<usize>(), 1);
    require_population(
        &selected_report(&dataset, &[UsJurisdiction::Wisconsin])?,
        1,
        0,
        1,
    )?;
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    Ok(())
}

#[test]
fn historical_coverage_keeps_candidate_counts_without_selected_acceptance_filtering() -> TestResult
{
    let fixture = source_births()?;
    let dataset = crate::export::ExportDataset::load_public_identity_fixture(&fixture.store)?;
    let scanned = Scanned::of(&dataset)?;
    check!(eq; scanned.athletes.len(), 3);
    check!(eq; coverage_report(&dataset, None)?.read.athletes, 3);
    let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[])?;
    let report = coverage_report_for_derivation(&derivation)?;
    check!(report
        .notes
        .iter()
        .any(|note| note.starts_with("retained canonical population reconciliation: raw [",)));
    check!(!report
        .notes
        .iter()
        .any(|note| note.starts_with("selected identity candidate audit:")));
    Ok(())
}

#[test]
fn selected_scan_and_pr_tally_refuse_a_missing_public_identity_view() -> TestResult {
    let fixture = source_births()?;
    let mut dataset = crate::export::ExportDataset::load_public_identity_fixture(&fixture.store)?;
    dataset.bind_selection_for_test(
        census_domain::model::ProfileId::parse(&"a".repeat(64))?,
        census_domain::model::ProfileDate::parse("2026-10-14")?,
    )?;
    check!(Scanned::of(&dataset).is_err());
    let historical = crate::export::ExportDataset::load_public_identity_fixture(&fixture.store)?;
    let mut report = coverage_report(&historical, None)?;
    check!(pr_support::tally_pr_support(&mut report, &dataset, None, &HashSet::new()).is_err());
    Ok(())
}

#[test]
fn unresolved_current_affiliations_exclude_accepted_people_without_counting_unknown_candidates(
) -> TestResult {
    let fixture = source_births()?;
    let dataset = fixture::unresolved_dataset(&fixture.store)?;
    let report = selected_report(&dataset, &[])?;
    check!(eq; report.raw_observations, 3);
    check!(eq; report.read.athletes, 0);
    check!(eq; report.off_cohort_athletes, 0);
    check!(report.notes.iter().any(|note| note ==
        "selected accepted people reconciliation: retained=2 = publication=0 + known outside requested jurisdictions=0 + current affiliation/evidence/cohort excluded=2"));
    check!(report.notes.iter().any(|note| note.contains(
        "retained_candidate_records=3 accepted_candidate_records=2 withheld_or_unknown_candidate_records=1 absent_candidate_records=0",
    )));
    let standalone = coverage_report(&dataset, None)?;
    check!(eq; standalone.read, report.read);
    check!(eq; standalone.notes, report.notes);
    report.reconcile()?;
    Ok(())
}
