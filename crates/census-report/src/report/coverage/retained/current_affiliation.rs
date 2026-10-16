use crate::export::CurrentAffiliationView;
use crate::report::{
    CoverageGap, CoverageReport, CoverageTotals, Derivation, JurisdictionCoverage,
};
use census_domain::model::CanonicalAthlete;
use std::collections::{BTreeMap, HashSet};

pub(super) fn append(
    report: &mut CoverageReport,
    derivation: &Derivation<'_>,
    raw: &[JurisdictionCoverage],
    gaps: &[CoverageGap],
) {
    super::append_outside(report, derivation, raw);
    super::append_gaps(report, derivation, gaps);
    match derivation.current_affiliations() {
        CurrentAffiliationView::Historical => {}
        CurrentAffiliationView::Selected(selected) => {
            report.notes.push("Retained stored-placement diagnostics count accepted people using selected identity authority; withheld or unknown candidate records remain candidate audit evidence. Stored school placement differs from selected current-affiliation geography; raw PR support is unmeasured and produces no missing-PR-support conclusion".into());
            report.notes.push(format!(
                "current affiliation geography: as_of={} profile={} input_generation={}",
                selected.as_of().as_str(),
                selected.profile_id(),
                selected.input_generation(),
            ));
            let mut counts = BTreeMap::new();
            for (athlete, reason) in derivation.affiliation_exclusions() {
                let key = (reason.as_str(), selected.resolution(athlete).status());
                let count = counts.entry(key).or_insert(0_usize);
                *count = count.saturating_add(1);
            }
            for ((reason, status), count) in counts {
                report.notes.push(format!(
                    "retained current affiliation exclusions: reason={reason} status={status} athletes={count}",
                ));
            }
        }
    }
}

pub(super) fn outside_totals(
    derivation: &Derivation<'_>,
    raw: &[JurisdictionCoverage],
) -> CoverageTotals {
    match derivation.current_affiliations() {
        CurrentAffiliationView::Historical => super::totals(raw.iter().filter(|row| {
            !crate::report::derivation::in_run_scope(row.jurisdiction, derivation.jurisdictions())
        })),
        CurrentAffiliationView::Selected(_) => selected_outside_totals(derivation),
    }
}

fn selected_outside_totals(derivation: &Derivation<'_>) -> CoverageTotals {
    let outside: HashSet<_> = derivation
        .outside_athletes()
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    let aliases = derivation.athlete_aliases();
    let performances = derivation
        .dataset()
        .performances
        .iter()
        .filter(|performance| {
            let subject = performance.athlete.as_str();
            outside.contains(aliases.get(subject).map_or(subject, String::as_str))
        })
        .count();
    CoverageTotals {
        schools: derivation.outside_schools().len(),
        athletes: outside.len(),
        coaches: derivation.outside_coaches().len(),
        meets: derivation.outside_meets().len(),
        performances,
    }
}

pub(super) fn off_cohort(derivation: &Derivation<'_>, athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| derivation.current_school_in_scope(athlete))
        .filter(|athlete| !super::super::in_requested_year(athlete, derivation.grad_year()))
        .count()
}

pub(super) fn placement_note(report: &mut CoverageReport, derivation: &Derivation<'_>) {
    report.notes.push(match derivation.current_affiliations() {
        CurrentAffiliationView::Historical => "UNPLACED is retained because jurisdiction is unknown; it does not certify membership in any requested jurisdiction or a completed jurisdiction denominator",
        CurrentAffiliationView::Selected(_) => "Unknown, conflicted and non-school current affiliations and unknown school jurisdictions are excluded from selected population; their evidence and individual exclusion reasons remain retained",
    }.into());
}

pub(super) fn outside_label(derivation: &Derivation<'_>) -> &'static str {
    match derivation.current_affiliations() {
        CurrentAffiliationView::Historical => "retained outside requested jurisdictions",
        CurrentAffiliationView::Selected(_) => {
            "retained historical/raw outside requested jurisdictions"
        }
    }
}

pub(super) fn gap_label(derivation: &Derivation<'_>) -> &'static str {
    match derivation.current_affiliations() {
        CurrentAffiliationView::Historical => "retained raw gap",
        CurrentAffiliationView::Selected(_) => "retained historical/raw gap",
    }
}
