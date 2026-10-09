use super::{row_locator, schedule, Harness, TestResult, EMPTY, FETCHED};
use crate::CollectionDisposition;

#[tokio::test]
async fn full_iso_horizon_excludes_future_but_retains_unknown_dates_as_owed() -> TestResult {
    let page = schedule(
        "September",
        &[
            ("Tue. 1", "Same-day admitted", "University of Minnesota"),
            ("Wed. 2", "Future retained", "Unknown venue"),
            ("Thu. 31", "Invalid calendar day", "University of Minnesota"),
            ("4-5", "Published period", "University of Minnesota"),
            ("2026", "Year only", "University of Minnesota"),
        ],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let report = harness.run(&[2026], &[], None, "2026-09-01").await?;
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(
        eq;
        report.unfinished,
        vec![row_locator(3), row_locator(4), row_locator(5)]
    );
    let meets = harness.meets()?;
    check!(
        eq;
        meets
            .iter()
            .map(|meet| meet.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Same-day admitted"]
    );
    check!(eq; meets[0].date, "2026-09-01");
    check!(eq; meets[0].evidence[0].observed_on, FETCHED);
    let mut raw = harness.raw()?;
    raw.sort_by_key(|row| row["capture"]["ordinal"].as_u64());
    check!(
        eq;
        raw.iter()
            .map(|row| row["date"].as_str())
            .collect::<Vec<_>>(),
        vec![
            Some("2026-09-01"),
            Some("2026-09-02"),
            Some("2026-09-31"),
            Some("4-5"),
            Some("2026"),
        ]
    );
    check!(raw
        .iter()
        .all(|row| row["capture"]["fetched_at"] == FETCHED));
    Ok(())
}

#[tokio::test]
async fn a_later_horizon_replays_previously_excluded_rows_without_relabeling_capture_time(
) -> TestResult {
    let page = schedule(
        "September",
        &[("2", "After first snapshot", "University of Minnesota")],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let first = harness.run(&[2026], &[], None, "2026-09-01").await?;
    check!(eq; first.disposition, CollectionDisposition::Complete);
    check!(harness.meets()?.is_empty());
    let second = harness.run(&[2026], &[], None, "2026-09-02").await?;
    check!(eq; second.disposition, CollectionDisposition::Complete);
    let meets = harness.meets()?;
    check!(
        eq;
        meets
            .iter()
            .map(|meet| (meet.name.as_str(), meet.date.as_str()))
            .collect::<Vec<_>>(),
        vec![("After first snapshot", "2026-09-02")]
    );
    check!(eq; meets[0].evidence[0].observed_on, FETCHED);
    check!(harness
        .raw()?
        .iter()
        .all(|row| row["capture"]["fetched_at"] == FETCHED));
    Ok(())
}

#[tokio::test]
async fn default_calendar_windows_follow_snapshot_year_not_cohort_year() -> TestResult {
    let current = schedule(
        "January",
        &[("1", "Snapshot calendar", "University of Minnesota")],
    );
    let previous = schedule(
        "January",
        &[("1", "Previous calendar", "University of Minnesota")],
    );
    let harness = Harness::new(&current, Some(EMPTY))?;
    harness.seed(crate::wayzata::ScheduleSport::Track, 2025, &previous)?;
    harness.seed(crate::wayzata::ScheduleSport::CrossCountry, 2025, EMPTY)?;
    let report = harness.run(&[], &[], None, "2026-09-01").await?;
    check!(eq; report.disposition, CollectionDisposition::Complete);
    check!(
        eq;
        harness
            .meets()?
            .iter()
            .map(|meet| (meet.name.as_str(), meet.date.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("Previous calendar", "2025-01-01"),
            ("Snapshot calendar", "2026-01-01")
        ]
    );
    Ok(())
}
