//! Printing what a run reported: the national table, the per-jurisdiction table, and the exit code
//! that says whether the run finished clean.

use anyhow::{bail, Result};
use census_service::restate_services::{
    JurisdictionReport, JurisdictionSummary, NationalReport, SourcePlan,
};

/// A cell for a value a report may predate: an unrecorded denominator is unknown, not zero.
pub(crate) fn cell(value: Option<usize>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "?".to_string(),
    }
}

/// The owed sources of one run, in plan order: the slugs an operator has to see, because a plan
/// that is recorded and never printed reads as a run that swept everything.
pub(crate) fn owed_sources(plan: &SourcePlan) -> String {
    plan.refused
        .iter()
        .map(|refusal| refusal.slug.as_str())
        .collect::<Vec<&str>>()
        .join(",")
}

/// The fold's owed total: unknown if any row is unknown, because a sum over known rows alone would
/// understate the work the run left unfinished.
pub(crate) fn owed_total(rows: &[JurisdictionSummary]) -> Option<usize> {
    rows.iter().map(|row| row.rosters_owed).sum()
}

/// How many jurisdictions were refused, on the same terms as [`owed_total`].
pub(crate) fn blocked_count(rows: &[JurisdictionSummary]) -> Option<usize> {
    rows.iter().try_fold(0_usize, |count, row| {
        row.blocked
            .map(|blocked| count.saturating_add(usize::from(blocked)))
    })
}

pub(crate) fn print_national(report: &NationalReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!(
        "national {} revision {} observed {}",
        report.season.short(),
        report.revision.get(),
        report.today
    );
    println!(
        "{:>3}  {:>8}  {:>7}  {:>7}  {:>7}  {:>9}  {:>8}  blocked",
        "st", "teams", "walked", "had", "owed", "athletes", "co2027"
    );
    for summary in &report.jurisdictions {
        // The `owed` column is the one an operator cannot reconstruct from the others: a state whose
        // host refused requests walks a handful of rosters and leaves the rest unfinished, and that
        // is a coverage gap rather than a small state. `?` is a report written before the column
        // existed — never printed as `0`.
        println!(
            "{:>3}  {:>8}  {:>7}  {:>7}  {:>7}  {:>9}  {:>8}  {}",
            summary.jurisdiction.code(),
            summary.teams,
            summary.rosters_done,
            summary.rosters_skipped,
            cell(summary.rosters_owed),
            summary.athletes,
            summary.class_of_2027,
            match summary.blocked {
                Some(true) => "refused",
                Some(false) => "",
                None => "?",
            },
        );
    }
    println!(
        "total: teams {} · athletes {} · co2027 {} · jurisdictions done {} · failed {} · owed {} · blocked {}",
        report.teams_total,
        report.athletes_total,
        report.class_of_2027_total,
        report.jurisdictions.len(),
        report.failures.len(),
        cell(owed_total(&report.jurisdictions)),
        cell(blocked_count(&report.jurisdictions)),
    );
    for failure in &report.failures {
        println!(
            "failed {} {} {}",
            failure.jurisdiction.code(),
            failure.identity,
            failure.error
        );
    }
    Ok(())
}

/// Exit non-zero when the fold carries failed jurisdictions.
///
/// The report is the artifact and it already printed — a failed jurisdiction's error text is in
/// those rows — but a shell that only sees the exit code must not read a run with failures as a
/// successful national census. `--detach` returns before this: a submission that has not drained
/// yet has no verdict to report, and inventing one would be worse than saying nothing.
pub(crate) fn failure_exit(report: &NationalReport) -> Result<()> {
    if report.failures.is_empty() {
        return Ok(());
    }
    bail!(
        "{} jurisdiction(s) failed; the `failed` rows above name each identity and its error",
        report.failures.len()
    )
}

pub(crate) fn print_jurisdiction(report: &JurisdictionReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!(
        "{} finished {} · stages {}",
        report.identity,
        report.completed_at,
        if report.stages_run.is_empty() {
            "none owed".to_string()
        } else {
            report.stages_run.join(",")
        }
    );
    println!(
        "teams {} · rosters {}/{} skipped {} · athletes {} · co2027 {} (boys {} girls {}) · blocked {}",
        report.teams,
        report.rosters.rosters_done,
        report.rosters.teams,
        report.rosters.rosters_skipped,
        report.rosters.athletes,
        report.rosters.class_of_2027,
        report.rosters.class_of_2027_boys,
        report.rosters.class_of_2027_girls,
        report.rosters.blocked,
    );
    for error in &report.rosters.errors {
        println!("error {error}");
    }
    // The plan's owed sources, printed on every run: a refusal is not a failure, so without this
    // line a run that owed ten sources reads exactly like a run that swept them.
    if !report.plan.refused.is_empty() {
        // The plan's own counts: a refusal is not a failure, so the line names what a run owed.
        let refused = report.plan.refused.len();
        let sweepable = report.plan.sweepable.len();
        let planned = refused.saturating_add(sweepable);
        println!(
            "sweepable {sweepable} · owed {refused} of {planned} planned · {}",
            owed_sources(&report.plan)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The segment is built from the plan as the journal records it, so what an operator reads is
    /// the run's own record — refusals in plan order, none dropped.
    #[test]
    fn owed_sources_are_the_plans_refusals_in_order() {
        let plan: SourcePlan = serde_json::from_value(serde_json::json!({
            "sweepable": ["milesplit"],
            "refused": [
                {"slug": "athleticnet", "reason": "no run stage sweeps this source per jurisdiction"},
                {"slug": "wiaa", "reason": "no run stage sweeps this source per jurisdiction"}
            ]
        }))
        .expect("a plan recorded by a run reads back");
        assert_eq!(owed_sources(&plan), "athleticnet,wiaa");
    }

    /// A plan with nothing owed prints nothing at all, so a clean run stays one line.
    #[test]
    fn nothing_owed_is_an_empty_segment() {
        let plan: SourcePlan = serde_json::from_value(serde_json::json!({
            "sweepable": ["milesplit"],
            "refused": []
        }))
        .expect("a plan recorded by a run reads back");
        assert!(owed_sources(&plan).is_empty());
    }
}
