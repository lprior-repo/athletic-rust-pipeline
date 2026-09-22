//! What the workflow commands must report: a failed jurisdiction exits non-zero, a blocked row
//! owes the index it did not walk, and a total is unknown rather than smaller when a row predates
//! its denominator.

use super::*;
use midwest_census::restate_services::{JurisdictionSummary, NationalFailure};

fn summary(jurisdiction: UsJurisdiction, teams: usize) -> JurisdictionSummary {
    JurisdictionSummary {
        jurisdiction,
        identity: format!("jurisdiction:{}:2026-27:1", jurisdiction.code()),
        teams,
        rosters_done: teams,
        rosters_skipped: 0,
        rosters_owed: Some(0),
        blocked: Some(false),
        athletes: teams * 2,
        class_of_2027: teams,
    }
}

/// A state whose host refused requests: a few rosters walked, the rest of the index owed.
fn blocked_summary(jurisdiction: UsJurisdiction, teams: usize) -> JurisdictionSummary {
    JurisdictionSummary {
        rosters_done: 49,
        rosters_skipped: 25,
        rosters_owed: Some(teams - 74),
        blocked: Some(true),
        athletes: 5924,
        class_of_2027: 1439,
        ..summary(jurisdiction, teams)
    }
}

fn report(failures: Vec<NationalFailure>) -> NationalReport {
    NationalReport {
        season: SchoolYear(2026),
        revision: Revision(1),
        jurisdictions: vec![summary(UsJurisdiction::Wisconsin, 7)],
        failures,
        teams_total: 7,
        athletes_total: 14,
        class_of_2027_total: 7,
        today: "2026-09-22".to_string(),
    }
}

#[test]
fn a_national_run_without_failures_exits_successfully() {
    assert!(failure_exit(&report(Vec::new())).is_ok());
}

#[test]
fn a_national_run_with_a_failed_jurisdiction_exits_non_zero() {
    // A failed jurisdiction is not the same observable as a jurisdiction that ran and found a
    // blocked source: the report carries its error text, and the exit code has to say the
    // census did not cover what it claimed.
    let failures = vec![NationalFailure {
        jurisdiction: UsJurisdiction::SouthDakota,
        identity: "jurisdiction:SD:2026-27:1".to_string(),
        error: "terminal: source refused every roster".to_string(),
    }];
    let error = failure_exit(&report(failures)).expect_err("a failure must be an error exit");
    assert!(
        error.to_string().contains("1 jurisdiction(s) failed"),
        "unexpected message: {error}"
    );
}

#[test]
fn a_national_report_prints_every_jurisdiction_row_it_is_given() {
    // The report is the artifact §46 is read from: one row per jurisdiction, and the fold's
    // totals are the sums of the rows, never a separate number that can disagree with them.
    let report = report(Vec::new());
    assert_eq!(report.jurisdictions.len(), 1);
    assert_eq!(
        report.teams_total,
        report
            .jurisdictions
            .iter()
            .map(|row| row.teams)
            .sum::<usize>()
    );
    assert_eq!(
        report.class_of_2027_total,
        report
            .jurisdictions
            .iter()
            .map(|row| row.class_of_2027)
            .sum::<usize>()
    );
}

/// The row an operator reads when a state's host refused requests: the numbers must add up to the
/// index, so a blocked state cannot be mistaken for a small one.
#[test]
fn a_blocked_row_owes_the_index_it_did_not_walk() {
    let row = blocked_summary(UsJurisdiction::Texas, 2423);
    assert_eq!(row.blocked, Some(true));
    assert_eq!(row.rosters_done, 49);
    assert_eq!(row.rosters_skipped, 25);
    assert_eq!(row.rosters_owed, Some(2349));
    assert_eq!(
        row.rosters_done + row.rosters_skipped + row.rosters_owed.expect("recorded"),
        row.teams,
        "walked + already-held + owed must reconstruct the team index"
    );
}

#[test]
fn a_total_over_a_partly_recorded_report_is_unknown_not_smaller() {
    // A report written before the owed column existed, sitting beside one that has it: summing
    // the known rows alone would print a national total that understates the unfinished walk,
    // which is exactly how a coverage gap becomes invisible.
    let mut report = report(Vec::new());
    let mut old = summary(UsJurisdiction::Utah, 172);
    old.rosters_owed = None;
    old.blocked = None;
    report.jurisdictions = vec![blocked_summary(UsJurisdiction::Texas, 2423), old];
    assert_eq!(owed_total(&report.jurisdictions), None);
    assert_eq!(blocked_count(&report.jurisdictions), None);
    assert_eq!(cell(owed_total(&report.jurisdictions)), "?");
}

#[test]
fn totals_sum_when_every_row_records_the_denominator() {
    let mut report = report(Vec::new());
    report.jurisdictions = vec![
        blocked_summary(UsJurisdiction::Texas, 2423),
        summary(UsJurisdiction::Utah, 172),
    ];
    assert_eq!(owed_total(&report.jurisdictions), Some(2349));
    assert_eq!(blocked_count(&report.jurisdictions), Some(1));
    assert_eq!(cell(owed_total(&report.jurisdictions)), "2349");
}

#[test]
fn a_report_carrying_a_blocked_row_still_exits_successfully() {
    // A refusal is a source condition, not a run failure: the run reached a terminal state for
    // the jurisdiction and the work stays owed. Failing the exit code here would make every
    // census run over a host that rate-limits read as failed.
    let mut report = report(Vec::new());
    report.jurisdictions = vec![blocked_summary(UsJurisdiction::Texas, 2423)];
    assert!(failure_exit(&report).is_ok());
}
