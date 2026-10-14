use crate::cli::gather::{TeamsCollectionError, TeamsOutcomes};
use census_crawl::{AdapterReport, CollectionDisposition as Disposition, CrawlError};
use census_domain::UsJurisdiction::{Minnesota, Wisconsin};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn report(rows: u64, disposition: Disposition) -> AdapterReport {
    let mut report = AdapterReport::new("milesplit", "teams");
    report.rows = rows;
    report.disposition = disposition;
    report
}

fn failure() -> census_service::census::SourceAccessFailure {
    CrawlError::Schema {
        url: "https://wi.milesplit.com/teams".to_string(),
        detail: "index publishes conflicting application ownership".to_string(),
    }
    .into()
}

#[test]
fn an_all_success_team_collection_finishes_ok() -> TestResult {
    let mut outcomes = TeamsOutcomes::default();
    check!(eq; outcomes.record_report(Wisconsin, report(5, Disposition::Complete)),
        "WI\tteams=5\toutcome=Complete\terrors=0\tunfinished=0");
    check!(eq; outcomes.record_report(Minnesota, report(3, Disposition::Complete)),
        "MN\tteams=3\toutcome=Complete\terrors=0\tunfinished=0");
    check!(eq; outcomes.disposition(), Disposition::Complete);
    let json: serde_json::Value = serde_json::from_str(&outcomes.json()?)?;
    check!(eq; json.pointer("/jurisdictions/1/report/rows"), Some(&serde_json::json!(3)));
    outcomes.finish()?;
    Ok(())
}

#[test]
fn failed_team_collection_preserves_a_typed_aggregate_error() -> TestResult {
    let mut outcomes = TeamsOutcomes::default();
    check!(eq; outcomes.record_failure(Wisconsin, &failure()), "WI\tteams=error\toutcome=Failed");
    let json: serde_json::Value = serde_json::from_str(&outcomes.json()?)?;
    check!(eq; json.get("disposition"), Some(&serde_json::json!("failed")));
    check!(eq; json.pointer("/jurisdictions/0/failure/status"), Some(&serde_json::json!("terminal")));
    check!(eq; json.pointer("/jurisdictions/0/failure/message"), Some(&serde_json::json!(failure().to_string())));
    let error = outcomes
        .finish()
        .err()
        .ok_or("all-failed collection succeeded")?;
    let typed = error
        .downcast_ref::<TeamsCollectionError>()
        .ok_or("opaque failure")?;
    check!(eq; typed.failed, vec![Wisconsin]);
    check!(eq; typed.incomplete, Vec::<census_domain::UsJurisdiction>::new());
    check!(eq; typed.disposition, Disposition::Failed);
    check!(error.to_string().contains("failed [WI]"));
    Ok(())
}

#[test]
fn a_mixed_team_collection_keeps_success_and_names_the_failed_state() -> TestResult {
    let mut outcomes = TeamsOutcomes::default();
    outcomes.record_failure(Wisconsin, &failure());
    outcomes.record_report(Minnesota, report(12, Disposition::Complete));
    let json: serde_json::Value = serde_json::from_str(&outcomes.json()?)?;
    check!(eq; json.get("disposition"), Some(&serde_json::json!("partial")));
    check!(eq; json.pointer("/jurisdictions/1/report/rows"), Some(&serde_json::json!(12)));
    check!(eq; json.pointer("/jurisdictions/1/report/disposition"), Some(&serde_json::json!("complete")));
    let error = outcomes
        .finish()
        .err()
        .ok_or("mixed collection succeeded")?;
    let typed = error
        .downcast_ref::<TeamsCollectionError>()
        .ok_or("opaque failure")?;
    check!(eq; typed.failed, vec![Wisconsin]);
    check!(eq; typed.disposition, Disposition::Partial);
    Ok(())
}

#[test]
fn a_partial_team_collection_is_not_success() -> TestResult {
    let mut partial = report(5, Disposition::Partial);
    partial.errors = 1;
    partial.unfinished = vec!["https://wi.milesplit.com/teams#row-20001".to_string()];
    let mut outcomes = TeamsOutcomes::default();
    check!(eq; outcomes.record_report(Wisconsin, partial),
        "WI\tteams=5\toutcome=Partial\terrors=1\tunfinished=1");
    let json: serde_json::Value = serde_json::from_str(&outcomes.json()?)?;
    check!(eq; json.pointer("/jurisdictions/0/report/unfinished/0"),
        Some(&serde_json::json!("https://wi.milesplit.com/teams#row-20001")));
    let error = outcomes
        .finish()
        .err()
        .ok_or("partial collection succeeded")?;
    let typed = error
        .downcast_ref::<TeamsCollectionError>()
        .ok_or("opaque failure")?;
    check!(eq; typed.incomplete, vec![Wisconsin]);
    check!(eq; typed.disposition, Disposition::Partial);
    Ok(())
}

#[test]
fn a_transient_team_failure_reports_the_single_attempt_without_claiming_exhaustion() -> TestResult {
    let error = CrawlError::Fetch(census_crawl::net::FetchError::Http {
        status: 503,
        url: "https://wi.milesplit.com/teams".to_string(),
    })
    .into();
    let mut outcomes = TeamsOutcomes::default();
    outcomes.record_failure(Wisconsin, &error);
    let json: serde_json::Value = serde_json::from_str(&outcomes.json()?)?;
    check!(eq; json.pointer("/jurisdictions/0/failure/status"), Some(&serde_json::json!("interrupted")));
    check!(eq; json.pointer("/jurisdictions/0/failure/attempts"), Some(&serde_json::json!(1)));
    check!(eq; outcomes.disposition(), Disposition::Failed);
    Ok(())
}

#[test]
fn team_access_retention_failure_keeps_original_retry_status_and_both_diagnostics() -> TestResult {
    let error = census_service::census::SourceAccessFailure::retention(
        CrawlError::Fetch(census_crawl::net::FetchError::Http {
            status: 429,
            url: "https://wi.milesplit.com/teams".to_string(),
        }),
        CrawlError::Resource {
            resource: "source access snapshot",
            requested: 4097,
            limit: 4096,
        },
    );
    let expected = error.to_string();
    let mut outcomes = TeamsOutcomes::default();
    outcomes.record_failure(Wisconsin, &error);
    let json: serde_json::Value = serde_json::from_str(&outcomes.json()?)?;
    check!(eq; json.pointer("/jurisdictions/0/failure/status"), Some(&serde_json::json!("interrupted")));
    check!(eq; json.pointer("/jurisdictions/0/failure/attempts"), Some(&serde_json::json!(1)));
    check!(eq; json.pointer("/jurisdictions/0/failure/message"), Some(&serde_json::json!(expected)));
    check!(expected.contains("429") && expected.contains("source access snapshot"));
    check!(eq; outcomes.disposition(), Disposition::Failed);
    Ok(())
}
