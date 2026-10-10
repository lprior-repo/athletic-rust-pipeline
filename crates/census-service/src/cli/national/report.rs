use anyhow::{bail, Result};
use census_service::restate_services::{
    JurisdictionReport, JurisdictionSummary, NationalReport, SourcePlan,
};

pub(crate) fn owed_sources(plan: &SourcePlan) -> String {
    plan.refused
        .iter()
        .map(|refusal| refusal.slug.as_str())
        .collect::<Vec<&str>>()
        .join(",")
}

pub(crate) fn owed_total(rows: &[JurisdictionSummary]) -> usize {
    rows.iter().map(|row| row.rosters_remaining).sum()
}

pub(crate) fn blocked_count(rows: &[JurisdictionSummary]) -> usize {
    rows.iter().filter(|row| row.blocked).count()
}

fn print_national_row(summary: &JurisdictionSummary) {
    println!(
        "{:>3}  {:>9}  {:>9}  {:>7}  {:>9}  {:>7}  {:>8}  {}",
        summary.jurisdiction.code(),
        summary.rosters_total,
        summary.rosters_committed,
        summary.rosters_skipped,
        summary.rosters_remaining,
        summary.athletes,
        summary.class_of_2027,
        if summary.blocked { "refused" } else { "" },
    );
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
        "{:>3}  {:>9}  {:>9}  {:>7}  {:>9}  {:>7}  {:>9}  {:>8}  blocked",
        "st", "rosters", "committed", "held", "remaining", "athletes", "co2027", ""
    );
    for summary in &report.jurisdictions {
        print_national_row(summary);
    }
    println!(
        "total: rosters {} · committed {} · athletes {} · co2027 {} · jurisdictions done {} · failed {} · owed {} · remaining {} · blocked {}",
        report.rosters_total,
        report.jurisdictions.iter().map(|s| s.rosters_committed).sum::<usize>(),
        report.athletes_total,
        report.class_of_2027_total,
        report.jurisdictions.len(),
        report.failures.len(),
        report.owed.len(),
        owed_total(&report.jurisdictions),
        blocked_count(&report.jurisdictions),
    );
    for failure in &report.failures {
        println!(
            "failed {} {} {}",
            failure.jurisdiction.code(),
            failure.identity,
            failure.error
        );
    }
    for owed in &report.owed {
        println!(
            "owed {} {} stages {} · {}",
            owed.jurisdiction.code(),
            owed.identity,
            if owed.stages_run.is_empty() {
                "none".to_string()
            } else {
                owed.stages_run.join(",")
            },
            owed.reasons.join(" | ")
        );
    }
    if let Some(join) = &report.school_address {
        println!(
            "school addresses: {} linked · {} already linked · {} review · {} no match · {} evidence missing · {} refused · generations {}",
            join.counters.linked,
            join.counters.already_linked,
            join.counters.review,
            join.counters.no_match,
            join.counters.evidence_missing,
            join.counters.refused,
            join.generation,
        );
    }
    Ok(())
}

pub(crate) fn failure_exit(report: &NationalReport) -> Result<()> {
    if report.failures.is_empty() && report.owed.is_empty() {
        return Ok(());
    }
    bail!(
        "{} jurisdiction(s) failed and {} still owe source work; the `failed` and `owed` rows above name each identity, its error and its owed stages",
        report.failures.len(),
        report.owed.len()
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
        "teams {} · rosters {}/{} held {} · athletes {} · co2027 {} (boys {} girls {}) · blocked {}",
        report.teams,
        report.rosters.rosters_committed,
        report.rosters.rosters_total,
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
    if !report.plan.refused.is_empty() {
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

    #[test]
    fn owed_sources_are_the_plans_refusals_in_order() -> Result<()> {
        let plan: SourcePlan = serde_json::from_value(serde_json::json!({
            "sweepable": ["milesplit"],
            "refused": [
                {"slug": "athleticnet", "reason": "no run stage sweeps this source per jurisdiction"},
                {"slug": "wiaa", "reason": "no run stage sweeps this source per jurisdiction"}
            ]
        }))?;
        let owed = owed_sources(&plan);
        anyhow::ensure!(
            owed == "athleticnet,wiaa",
            "owed sources: left={owed:?}, right=\"athleticnet,wiaa\""
        );
        Ok(())
    }

    #[test]
    fn nothing_owed_is_an_empty_segment() -> Result<()> {
        let plan: SourcePlan = serde_json::from_value(serde_json::json!({
            "sweepable": ["milesplit"],
            "refused": []
        }))?;
        let owed = owed_sources(&plan);
        anyhow::ensure!(owed.is_empty(), "expected no owed sources, got {owed:?}");
        Ok(())
    }
}
