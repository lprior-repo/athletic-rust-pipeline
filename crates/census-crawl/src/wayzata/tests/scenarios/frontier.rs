use super::{observed_meet_names, row_locator, schedule, Harness, TestResult, EMPTY};
use crate::wayzata::{schedule_url, ScheduleSport};
use crate::CollectionDisposition;

#[tokio::test]
async fn limited_pass_keeps_prefix_durable_and_unlimited_resume_admits_each_remaining_row_once(
) -> TestResult {
    let page = schedule(
        "January",
        &[
            ("1", "First admitted", "University of Minnesota"),
            ("2", "Second admitted", "University of Minnesota"),
        ],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let first = harness.run(&[2026], &[], Some(1), "2026-09-20").await?;
    check!(eq; first.disposition, CollectionDisposition::Partial);
    check!(
        eq;
        first.unfinished,
        vec![
            row_locator(2),
            schedule_url(ScheduleSport::CrossCountry, 2026)
        ]
    );
    check!(eq; observed_meet_names(&harness)?, vec!["First admitted"]);
    let resumed = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; resumed.disposition, CollectionDisposition::Complete);
    check!(eq; resumed.rows, 1);
    check!(
        eq;
        observed_meet_names(&harness)?,
        vec!["First admitted", "Second admitted"]
    );
    let again = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; again.disposition, CollectionDisposition::Complete);
    check!(eq; again.rows, 0);
    check!(
        eq;
        observed_meet_names(&harness)?,
        vec!["First admitted", "Second admitted"]
    );
    Ok(())
}

#[tokio::test]
async fn a_later_page_failure_does_not_roll_back_an_admitted_prefix() -> TestResult {
    let page = schedule(
        "January",
        &[("1", "Durable before failure", "University of Minnesota")],
    );
    let harness = Harness::new(&page, None)?;
    let first = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; first.disposition, CollectionDisposition::Partial);
    check!(
        eq;
        first.unfinished,
        vec![schedule_url(ScheduleSport::CrossCountry, 2026)]
    );
    check!(
        eq;
        harness
            .meets()?
            .iter()
            .map(|meet| meet.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Durable before failure"]
    );
    harness.seed(ScheduleSport::CrossCountry, 2026, EMPTY)?;
    let resumed = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; resumed.disposition, CollectionDisposition::Complete);
    check!(eq; resumed.rows, 0);
    check!(
        eq;
        harness
            .meets()?
            .iter()
            .map(|meet| meet.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Durable before failure"]
    );
    Ok(())
}

#[tokio::test]
async fn future_rows_are_not_owed_when_an_admitted_row_consumes_the_limit() -> TestResult {
    let page = schedule(
        "September",
        &[
            ("1", "Admitted at limit", "University of Minnesota"),
            ("2", "Future with unknown geography", "Unknown venue"),
        ],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let report = harness.run(&[2026], &[], Some(1), "2026-09-01").await?;
    check!(
        eq;
        report.unfinished,
        vec![schedule_url(ScheduleSport::CrossCountry, 2026)]
    );
    check!(harness
        .raw()?
        .iter()
        .any(|row| row["name"] == "Future with unknown geography" && row["date"] == "2026-09-02"));
    check!(
        eq;
        harness
            .meets()?
            .iter()
            .map(|meet| meet.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Admitted at limit"]
    );
    Ok(())
}

#[tokio::test]
async fn missing_or_truncated_tables_cannot_certify_published_empty() -> TestResult {
    let harness = Harness::new("<html>login</html>", Some(EMPTY))?;
    let missing = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; missing.disposition, CollectionDisposition::Partial);
    check!(
        eq;
        missing.unfinished,
        vec![schedule_url(ScheduleSport::Track, 2026)]
    );
    harness.seed(
        ScheduleSport::Track,
        2026,
        "<table class=\"schedule\"><tr class=\"event-row\"><td>1</td></table>",
    )?;
    let truncated = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; truncated.disposition, CollectionDisposition::Partial);
    check!(
        eq;
        truncated.unfinished,
        vec![schedule_url(ScheduleSport::Track, 2026)]
    );
    harness.seed(ScheduleSport::Track, 2026, EMPTY)?;
    let published_empty = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; published_empty.disposition, CollectionDisposition::Complete);
    check!(harness.meets()?.is_empty());
    Ok(())
}

#[tokio::test]
async fn a_historical_url_only_journal_does_not_hide_a_current_schedule() -> TestResult {
    let page = schedule(
        "January",
        &[("1", "Current projection", "University of Minnesota")],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let url = schedule_url(ScheduleSport::Track, 2026);
    harness.store.journal_done(
        "wayzata_schedule",
        &url,
        &serde_json::json!({
            "url": url, "parser": 3, "sport": "track", "year": 2026, "rows": 1,
        }),
    )?;
    let report = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; report.disposition, CollectionDisposition::Complete);
    check!(
        eq;
        harness
            .meets()?
            .iter()
            .map(|meet| meet.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Current projection"]
    );
    Ok(())
}

#[tokio::test]
async fn missing_capture_time_cannot_certify_a_published_empty_schedule() -> TestResult {
    let harness = Harness::new(EMPTY, Some(EMPTY))?;
    let url = schedule_url(ScheduleSport::Track, 2026);
    super::super::seed_cache_at(&harness.store.http_cache_dir(), &url, EMPTY, "")?;
    let report = harness.run(&[2026], &[], None, "2026-09-20").await?;
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(eq; report.unfinished, vec![url]);
    check!(harness.meets()?.is_empty());
    Ok(())
}
