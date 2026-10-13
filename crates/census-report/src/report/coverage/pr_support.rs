use super::{CoverageGap, CoverageReport, GapClass, JurisdictionBucket, Scope};
use crate::export::ExportDataset;
use std::collections::HashSet;

pub(super) fn apply_pr_support(
    report: &mut CoverageReport,
    dataset: &ExportDataset,
    grad_year: Option<i16>,
) -> super::ReportResult<()> {
    let bests = crate::bests::build_from_dataset(
        dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year,
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;
    let supported: HashSet<&str> = bests
        .iter()
        .filter(|selection| selection.result.adjustment_kind.is_none())
        .map(|selection| selection.athlete_id().as_str())
        .collect();
    tally_pr_support(report, dataset, grad_year, &supported);
    write_pr_gaps(report);
    Ok(())
}

pub(super) fn tally_pr_support(
    report: &mut CoverageReport,
    dataset: &ExportDataset,
    grad_year: Option<i16>,
    supported: &HashSet<&str>,
) {
    let school_state = super::state::school_state_index(dataset.schools.values());
    let athletes =
        crate::report::derivation::collapse_athletes(&dataset.athletes, &dataset.canonical_aliases);
    for athlete in &athletes {
        if !super::state::in_requested_year(athlete, grad_year) {
            continue;
        }
        if !supported.contains(athlete.id.as_str()) {
            continue;
        }
        let bucket = super::state::jurisdiction_of(&school_state, athlete.school.as_str());
        bump_pr_support(report, bucket);
    }
}

fn bump_pr_support(report: &mut CoverageReport, bucket: JurisdictionBucket) {
    if let Some(row) = report
        .jurisdictions
        .iter_mut()
        .find(|row| row.jurisdiction == bucket)
    {
        row.with_pr_support = row.with_pr_support.saturating_add(1);
    }
}

pub(super) fn write_pr_gaps(report: &mut CoverageReport) {
    for row in &report.jurisdictions {
        let count = row.with_performance.saturating_sub(row.with_pr_support);
        match report.gaps.iter_mut().find(|gap| {
            gap.jurisdiction == row.jurisdiction && gap.class == GapClass::MissingPrSupport
        }) {
            Some(gap) => gap.count = count,
            None if count > 0 => report.gaps.push(CoverageGap {
                jurisdiction: row.jurisdiction,
                class: GapClass::MissingPrSupport,
                unit: GapClass::MissingPrSupport.unit(),
                count,
            }),
            None => {}
        }
    }
    report
        .gaps
        .retain(|gap| gap.class != GapClass::MissingPrSupport || gap.count > 0);
}
