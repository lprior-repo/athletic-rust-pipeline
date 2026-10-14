use super::*;
use census_crawl::milesplit::ResultSetRequest;
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn meet() -> SourceMeetRef {
    SourceMeetRef {
        id: "milesplit:123".into(),
        source: "milesplit".into(),
        source_meet_id: "123".into(),
        jurisdiction: UsJurisdiction::Michigan,
        season: "xc".into(),
        year: 2026,
        name: "Owned meet".into(),
        date: Some("2026-09-01".into()),
        venue: String::new(),
        results_url: "https://mi.milesplit.com/meets/123/results".into(),
        observed_on: "2026-10-14".into(),
    }
}

fn options(url: &str) -> ResultSetOptions {
    ResultSetOptions {
        urls: vec![ResultSetRequest {
            url: url.into(),
            jurisdiction: UsJurisdiction::Michigan,
        }],
    }
}

#[test]
fn grant_target_matches_all_discovered_result_sets() -> TestResult {
    let selected = meet();
    let target = checked_target(
        &selected,
        &options("https://mi.milesplit.com/meets/123/results/5/raw"),
    )?;
    check!(eq; target.meet_id, "123");
    for foreign in [
        "https://mi.milesplit.com/meets/456/results/5/raw",
        "https://wi.milesplit.com/meets/123/results/5/raw",
    ] {
        check!(matches!(
            checked_target(&selected, &options(foreign)),
            Err(TargetAdmissionError::OwnerMismatch)
        ));
    }
    check!(matches!(
        checked_target(
            &selected,
            &options("https://example.com/meets/123/results/5/raw")
        ),
        Err(TargetAdmissionError::Unparseable)
    ));
    Ok(())
}
