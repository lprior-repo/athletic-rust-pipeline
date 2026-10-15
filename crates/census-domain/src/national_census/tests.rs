use super::*;

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

fn summary(
    jurisdiction: UsJurisdiction,
    rosters: usize,
    athletes: usize,
    cohort: usize,
) -> JurisdictionSummary {
    JurisdictionSummary {
        jurisdiction,
        identity: jurisdiction.code().to_string(),
        rosters_total: rosters,
        rosters_committed: rosters,
        rosters_remaining: 0,
        rosters_skipped: 0,
        blocked: false,
        athletes,
        class_of_2027: cohort,
    }
}

fn report() -> NationalReport {
    NationalReport {
        season: SchoolYear::DEFAULT,
        revision: Revision(1),
        jurisdictions: vec![summary(UsJurisdiction::Wisconsin, 2, 3, 2)],
        failures: Vec::new(),
        owed: Vec::new(),
        rosters_total: 2,
        athletes_total: 3,
        class_of_2027_total: 2,
        school_address: None,
        today: String::new(),
        completeness: NationalCompleteness::Final,
    }
}

fn set_metric(row: &mut JurisdictionSummary, metric: NationalMetric, value: usize) {
    match metric {
        NationalMetric::Rosters => row.rosters_total = value,
        NationalMetric::Athletes => row.athletes = value,
        NationalMetric::ClassOf2027 => row.class_of_2027 = value,
    }
}

fn failure(jurisdiction: UsJurisdiction) -> NationalFailure {
    NationalFailure {
        jurisdiction,
        identity: jurisdiction.code().to_string(),
        error: "retained source refusal".to_string(),
    }
}

fn owed(jurisdiction: UsJurisdiction) -> JurisdictionOwed {
    JurisdictionOwed {
        identity: jurisdiction.code().to_string(),
        jurisdiction,
        stages_run: Vec::new(),
        owed: vec!["teams".to_string()],
        reasons: vec!["source work remains".to_string()],
    }
}

#[test]
fn totals_accept_the_exact_limit_and_reject_each_metric_overflow() -> TestResult {
    for (metric, expected) in [
        (
            NationalMetric::Rosters,
            NationalTotals {
                rosters: usize::MAX,
                athletes: 0,
                class_of_2027: 0,
            },
        ),
        (
            NationalMetric::Athletes,
            NationalTotals {
                rosters: 0,
                athletes: usize::MAX,
                class_of_2027: 0,
            },
        ),
        (
            NationalMetric::ClassOf2027,
            NationalTotals {
                rosters: 0,
                athletes: 0,
                class_of_2027: usize::MAX,
            },
        ),
    ] {
        let mut boundary = summary(UsJurisdiction::Wisconsin, 0, 0, 0);
        set_metric(&mut boundary, metric, usize::MAX);
        check!(eq; NationalTotals::of(std::slice::from_ref(&boundary)), Ok(expected));

        let mut excess = summary(UsJurisdiction::Iowa, 0, 0, 0);
        set_metric(&mut excess, metric, 1);
        check!(eq; NationalTotals::of(&[boundary, excess]), Err(NationalReportError::TotalOverflow { metric }));
    }
    Ok(())
}

#[test]
fn each_reported_total_must_match_its_jurisdiction_summaries() -> TestResult {
    for (metric, expected) in [
        (NationalMetric::Rosters, 2),
        (NationalMetric::Athletes, 3),
        (NationalMetric::ClassOf2027, 2),
    ] {
        let mut given = report();
        match metric {
            NationalMetric::Rosters => given.rosters_total = 99,
            NationalMetric::Athletes => given.athletes_total = 99,
            NationalMetric::ClassOf2027 => given.class_of_2027_total = 99,
        }

        check!(eq; given.validate(), Err(NationalReportError::TotalMismatch { metric, expected, actual: 99 }));
    }
    Ok(())
}

#[test]
fn duplicate_jurisdictions_are_rejected_within_each_report_list() -> TestResult {
    let jurisdiction = UsJurisdiction::Wisconsin;
    for rows in [
        NationalRows::Jurisdictions,
        NationalRows::Failures,
        NationalRows::Owed,
    ] {
        let mut given = report();
        match rows {
            NationalRows::Jurisdictions => {
                given.jurisdictions = vec![
                    summary(jurisdiction, 1, 1, 1),
                    summary(jurisdiction, 1, 1, 1),
                ]
            }
            NationalRows::Failures => {
                given.failures = vec![failure(jurisdiction), failure(jurisdiction)]
            }
            NationalRows::Owed => given.owed = vec![owed(jurisdiction), owed(jurisdiction)],
        }

        check!(eq; given.validate(), Err(NationalReportError::Duplicate { rows, jurisdiction }));
    }
    Ok(())
}

#[test]
fn every_report_list_uses_jurisdiction_code_order() -> TestResult {
    let first = UsJurisdiction::Wisconsin;
    let second = UsJurisdiction::Iowa;
    for rows in [
        NationalRows::Jurisdictions,
        NationalRows::Failures,
        NationalRows::Owed,
    ] {
        let mut given = report();
        match rows {
            NationalRows::Jurisdictions => {
                given.jurisdictions = vec![summary(first, 1, 1, 1), summary(second, 1, 2, 1)]
            }
            NationalRows::Failures => given.failures = vec![failure(first), failure(second)],
            NationalRows::Owed => given.owed = vec![owed(first), owed(second)],
        }

        check!(eq; given.validate(), Err(NationalReportError::Unordered { rows }));
    }
    Ok(())
}

#[test]
fn one_jurisdiction_can_have_reported_results_and_other_unfinished_stages() -> TestResult {
    let mut given = report();
    given.failures.push(failure(UsJurisdiction::Wisconsin));
    given.owed.push(owed(UsJurisdiction::Wisconsin));

    check!(eq; given.validate(), Ok(()));
    Ok(())
}

#[test]
fn partial_report_cannot_claim_a_final_observation_date() -> TestResult {
    let mut given = report();
    given.completeness = NationalCompleteness::Partial;
    given.today = "2026-10-15".to_string();

    check!(eq; given.validate(), Err(NationalReportError::PartialMetadata));
    Ok(())
}

#[test]
fn partial_report_cannot_claim_completed_school_address_output() -> TestResult {
    let mut given = report();
    given.completeness = NationalCompleteness::Partial;
    given.school_address = Some(SchoolAddressJoinReply {
        mode: "verify".to_string(),
        generation: "generation".to_string(),
        report: "report.json".to_string(),
        outcomes: "outcomes.json".to_string(),
        counters: Counters::default(),
    });

    check!(eq; given.validate(), Err(NationalReportError::PartialMetadata));
    Ok(())
}

#[test]
fn partial_report_accepts_observed_counts_without_final_metadata() -> TestResult {
    let mut given = report();
    given.completeness = NationalCompleteness::Partial;

    check!(eq; given.validate(), Ok(()));
    Ok(())
}
