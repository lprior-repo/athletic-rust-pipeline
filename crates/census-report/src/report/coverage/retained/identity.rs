use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::report::{CoverageReport, CoverageTotals, Derivation, ReportResult};

pub(super) fn append(
    report: &mut CoverageReport,
    derivation: &Derivation<'_>,
    retained: CoverageTotals,
    outside: CoverageTotals,
    excluded: CoverageTotals,
) -> ReportResult<()> {
    if derivation.dataset().selected_profile_id().is_some() {
        append_candidates(report, derivation)?;
        report.notes.push(format!(
            "selected accepted people reconciliation: retained={} = publication={} + known outside requested jurisdictions={} + current affiliation/evidence/cohort excluded={}",
            retained.athletes, report.read.athletes, outside.athletes, excluded.athletes,
        ));
        report.notes.push(format!(
            "retained selected-authority population reconciliation: retained [{}] = publication [{}] + known outside requested jurisdictions [{}] + current affiliation/evidence/cohort excluded [{}]; athletes count accepted people, other columns count retained records",
            super::super::totals_text(&retained), super::super::totals_text(&report.read),
            super::super::totals_text(&outside), super::super::totals_text(&excluded),
        ));
    } else {
        report.notes.push(format!(
            "retained canonical population reconciliation: raw [{}] = publication [{}] + known outside requested jurisdictions [{}] + evidence/cohort excluded [{}]",
            super::super::totals_text(&retained), super::super::totals_text(&report.read),
            super::super::totals_text(&outside), super::super::totals_text(&excluded),
        ));
    }
    Ok(())
}

fn append_candidates(report: &mut CoverageReport, derivation: &Derivation<'_>) -> ReportResult<()> {
    let dataset = derivation.dataset();
    let population = IdentityPopulation::of(dataset)?;
    let count = |admission| {
        dataset
            .athletes
            .iter()
            .filter(|athlete| population.admission(athlete.id.as_str()) == admission)
            .count()
    };
    report.notes.push(format!(
        "selected identity candidate audit: retained_candidate_records={} accepted_candidate_records={} withheld_or_unknown_candidate_records={} absent_candidate_records={}; candidate records are not distinct accepted people",
        dataset.athletes.len(), count(IdentityAdmission::Accepted),
        count(IdentityAdmission::Withheld), count(IdentityAdmission::Absent),
    ));
    Ok(())
}
