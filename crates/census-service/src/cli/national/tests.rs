use super::*;
use census_domain::national_census::{JurisdictionOwed, JurisdictionSummary, NationalFailure};
#[path = "tests/publication.rs"]
mod publication;
#[path = "tests/resume.rs"]
mod resume;
#[path = "tests/seal_receipt.rs"]
mod seal_receipt;

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

fn owed_row(jurisdiction: UsJurisdiction) -> JurisdictionOwed {
    JurisdictionOwed {
        identity: format!("jurisdiction:{}:2026-27:1", jurisdiction.code()),
        jurisdiction,
        stages_run: vec!["rosters".to_string()],
        owed: vec!["rosters".to_string()],
        reasons: vec!["rosters: committed=1 remaining=9".to_string()],
    }
}

fn report(failures: Vec<NationalFailure>) -> NationalReport {
    NationalReport {
        season: SchoolYear::DEFAULT,
        revision: Revision(1),
        jurisdictions: vec![summary(UsJurisdiction::Wisconsin, 7)],
        failures,
        owed: Vec::new(),
        rosters_total: 7,
        athletes_total: 14,
        class_of_2027_total: 7,
        school_address: None,
        today: "2026-09-22".to_string(),
        completeness: Default::default(),
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

#[test]
fn a_report_that_still_owes_source_work_exits_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    let mut report = report(Vec::new());
    report.owed.push(owed_row(UsJurisdiction::SouthDakota));
    let error = match failure_exit(&report) {
        Err(error) => error,
        Ok(_) => return Err("owed source work returned success".into()),
    };
    if !error.to_string().contains("1 still owe source work") {
        return Err(format!("unexpected message: {error}").into());
    }
    Ok(())
}

#[test]
fn missing_completeness_decodes_as_final_for_backward_compatibility(
) -> Result<(), Box<dyn std::error::Error>> {
    let report: NationalReport = serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 1,
        "jurisdictions": [], "failures": [],
        "rosters_total": 0, "athletes_total": 0, "class_of_2027_total": 0,
        "today": "2026-09-22"
    }))?;
    check!(!report.is_partial());
    check!(eq; report.completeness_label(), "final");
    Ok(())
}

#[test]
fn partial_counters_are_observable_before_completion_and_converge_on_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    let partial: NationalReport = serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 1,
        "jurisdictions": [{
            "jurisdiction": "WI", "identity": "jurisdiction:WI:2026-27:1",
            "rosters_total": 7, "rosters_committed": 7, "rosters_remaining": 0,
            "rosters_skipped": 0, "blocked": false, "athletes": 14, "class_of_2027": 7
        }],
        "failures": [], "owed": [],
        "rosters_total": 7, "athletes_total": 14, "class_of_2027_total": 7,
        "today": "", "completeness": "partial"
    }))?;
    check!(partial.is_partial());
    check!(eq; partial.completeness_label(), "partial");
    check!(eq; partial.jurisdictions.len(), 1);
    check!(eq; partial.rosters_total, 7);
    check!(eq; partial.athletes_total, 14);
    check!(eq; partial.class_of_2027_total, 7);
    let final_report = report(Vec::new());
    check!(!final_report.is_partial());
    check!(eq; final_report.completeness_label(), "final");
    check!(eq; partial.rosters_total, final_report.rosters_total);
    check!(eq; partial.athletes_total, final_report.athletes_total);
    check!(eq;
        partial.class_of_2027_total,
        final_report.class_of_2027_total
    );
    Ok(())
}
