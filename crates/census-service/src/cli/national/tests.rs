use super::*;
use census_service::restate_services::{JurisdictionSummary, NationalFailure};

fn summary(jurisdiction: UsJurisdiction, teams: usize) -> JurisdictionSummary {
    JurisdictionSummary {
        jurisdiction,
        identity: format!("jurisdiction:{}:2026-27:1", jurisdiction.code()),
        rosters_total: teams,
        rosters_committed: teams,
        rosters_remaining: 0,
        rosters_skipped: 0,
        blocked: false,
        athletes: teams * 2,
        class_of_2027: teams,
    }
}

fn blocked_summary(jurisdiction: UsJurisdiction, teams: usize) -> JurisdictionSummary {
    JurisdictionSummary {
        rosters_committed: 49,
        rosters_skipped: 25,
        rosters_remaining: teams - 74,
        blocked: true,
        athletes: 5924,
        class_of_2027: 1439,
        ..summary(jurisdiction, teams)
    }
}

fn report(failures: Vec<NationalFailure>) -> NationalReport {
    NationalReport {
        season: SchoolYear::DEFAULT,
        revision: Revision(1),
        jurisdictions: vec![summary(UsJurisdiction::Wisconsin, 7)],
        failures,
        rosters_total: 7,
        athletes_total: 14,
        class_of_2027_total: 7,
        school_address: None,
        today: "2026-09-22".to_string(),
    }
}

#[test]
fn a_national_run_without_failures_exits_successfully() {
    assert!(failure_exit(&report(Vec::new())).is_ok());
}

#[test]
fn a_national_run_with_a_failed_jurisdiction_exits_non_zero(
) -> Result<(), Box<dyn std::error::Error>> {
    let failures = vec![NationalFailure {
        jurisdiction: UsJurisdiction::SouthDakota,
        identity: "jurisdiction:SD:2026-27:1".to_string(),
        error: "terminal: source refused every roster".to_string(),
    }];
    let error = match failure_exit(&report(failures)) {
        Err(error) => error,
        Ok(_) => return Err("failed jurisdiction returned success".into()),
    };
    if !error.to_string().contains("1 jurisdiction(s) failed") {
        return Err(format!("unexpected message: {error}").into());
    }
    Ok(())
}

#[test]
fn a_national_report_prints_every_jurisdiction_row_it_is_given() {
    let report = report(Vec::new());
    assert_eq!(report.jurisdictions.len(), 1);
    assert_eq!(
        report.rosters_total,
        report
            .jurisdictions
            .iter()
            .map(|row| row.rosters_total)
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

#[test]
fn a_blocked_row_owes_the_index_it_did_not_walk() {
    let row = blocked_summary(UsJurisdiction::Texas, 2423);
    assert!(row.blocked);
    assert_eq!(row.rosters_committed, 49);
    assert_eq!(row.rosters_skipped, 25);
    assert_eq!(row.rosters_remaining, 2349);
    assert_eq!(
        row.rosters_committed + row.rosters_skipped + row.rosters_remaining,
        row.rosters_total,
        "walked + already-held + owed must reconstruct the team index"
    );
}

#[test]
fn totals_sum_when_every_row_records_the_denominator() {
    let mut report = report(Vec::new());
    report.jurisdictions = vec![
        blocked_summary(UsJurisdiction::Texas, 2423),
        summary(UsJurisdiction::Utah, 172),
    ];
    assert_eq!(owed_total(&report.jurisdictions), 2349);
    assert_eq!(blocked_count(&report.jurisdictions), 1);
}

#[test]
fn a_report_carrying_a_blocked_row_still_exits_successfully() {
    let mut report = report(Vec::new());
    report.jurisdictions = vec![blocked_summary(UsJurisdiction::Texas, 2423)];
    assert!(failure_exit(&report).is_ok());
}
