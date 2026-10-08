use super::{row_locator, schedule, Harness, TestResult, EMPTY};
use crate::CollectionDisposition;
use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_store::Table;

#[tokio::test]
async fn requested_geography_uses_published_venues_not_school_name_heuristics() -> TestResult {
    let page = schedule(
        "January",
        &[
            ("1", "Wisconsin campus", "UW-River Falls"),
            ("2", "Minnesota campus", "University of Minnesota"),
            ("3", "School-shaped unknown", "River Falls HS"),
            (
                "4",
                "Nonexact campus label",
                "University of Minnesota Training Annex",
            ),
            ("5", "Explicit published state", "Local Park, WI"),
        ],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let school = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "River Falls High School",
        normalize_name("River Falls High School"),
        None,
    )
    .0;
    harness.store.append(Table::Schools, &school)?;
    let report = harness
        .run(&[2026], &[UsJurisdiction::Wisconsin], None, "2026-09-20")
        .await?;
    assert_eq!(report.disposition, CollectionDisposition::Partial);
    assert_eq!(report.unfinished, vec![row_locator(3), row_locator(4)]);
    assert_eq!(
        harness
            .meets()?
            .iter()
            .map(|meet| (meet.name.as_str(), meet.state))
            .collect::<Vec<_>>(),
        vec![
            ("Explicit published state", Some(UsJurisdiction::Wisconsin)),
            ("Wisconsin campus", Some(UsJurisdiction::Wisconsin)),
        ]
    );
    let raw = harness.raw()?;
    assert!(raw
        .iter()
        .any(|row| row["name"] == "School-shaped unknown" && row["venue"] == "River Falls HS"));
    assert!(raw
        .iter()
        .any(|row| row["name"] == "Minnesota campus" && row["venue"] == "University of Minnesota"));
    Ok(())
}

#[tokio::test]
async fn a_changed_requested_scope_replays_a_previously_excluded_meet() -> TestResult {
    let page = schedule(
        "January",
        &[("1", "Minnesota replay", "University of Minnesota")],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let excluded = harness
        .run(&[2026], &[UsJurisdiction::Wisconsin], None, "2026-09-20")
        .await?;
    assert_eq!(excluded.disposition, CollectionDisposition::Complete);
    assert!(harness.meets()?.is_empty());
    let admitted = harness
        .run(&[2026], &[UsJurisdiction::Minnesota], None, "2026-09-20")
        .await?;
    assert_eq!(admitted.disposition, CollectionDisposition::Complete);
    assert_eq!(
        harness
            .meets()?
            .iter()
            .map(|meet| (meet.name.as_str(), meet.state))
            .collect::<Vec<_>>(),
        vec![("Minnesota replay", Some(UsJurisdiction::Minnesota))]
    );
    Ok(())
}

#[tokio::test]
async fn row_content_cannot_impersonate_a_month_heading() -> TestResult {
    let page = schedule(
        "January",
        &[("1", "month-title Invitational", "University of Minnesota")],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let report = harness.run(&[2026], &[], None, "2026-09-20").await?;
    assert_eq!(report.disposition, CollectionDisposition::Complete);
    assert_eq!(
        harness
            .meets()?
            .iter()
            .map(|meet| (meet.name.as_str(), meet.date.as_str()))
            .collect::<Vec<_>>(),
        vec![("month-title Invitational", "2026-01-01")]
    );
    Ok(())
}

#[tokio::test]
async fn malformed_rows_keep_their_raw_metadata_and_do_not_hide_later_meets() -> TestResult {
    let page = schedule(
        "January",
        &[
            ("1", "Before held rows", "University of Minnesota"),
            ("", "", ""),
            ("3", "After held rows", "University of Minnesota"),
            ("4", "", "University of Minnesota"),
            ("5", "Missing venue", ""),
        ],
    );
    let harness = Harness::new(&page, Some(EMPTY))?;
    let report = harness.run(&[2026], &[], None, "2026-09-20").await?;
    assert_eq!(report.disposition, CollectionDisposition::Partial);
    assert_eq!(
        report.unfinished,
        vec![row_locator(2), row_locator(4), row_locator(5)]
    );
    assert_eq!(
        harness
            .meets()?
            .iter()
            .map(|meet| (meet.name.as_str(), meet.date.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("After held rows", "2026-01-03"),
            ("Before held rows", "2026-01-01")
        ]
    );
    let raw = harness.raw()?;
    assert!(raw.iter().any(|row| row["capture"]["ordinal"] == 2
        && row["date"] == ""
        && row["name"] == ""
        && row["venue"] == ""));
    assert!(raw.iter().any(|row| row["capture"]["ordinal"] == 4
        && row["date"] == "2026-01-04"
        && row["name"] == ""));
    assert!(raw.iter().any(|row| row["capture"]["ordinal"] == 5
        && row["name"] == "Missing venue"
        && row["venue"] == ""));
    Ok(())
}

#[tokio::test]
async fn valid_single_quoted_cells_preserve_apostrophes_and_provider_links() -> TestResult {
    let page = r#"<table class='table schedule'><tr class='month-title'><td>January</td></tr><tr class='event-row'><td class='date'>1</td><td class='awayteam'><span title="Saint Mary's Invitational">Display label</span></td><td class='hometeam'>University of Minnesota</td><td><a href='/links/knownid' aria-label="January 1 Saint Mary's Invitational">Results</a></td></tr></table>"#;
    let harness = Harness::new(page, Some(EMPTY))?;
    let report = harness.run(&[2026], &[], None, "2026-09-20").await?;
    assert_eq!(report.disposition, CollectionDisposition::Complete);
    let meets = harness.meets()?;
    assert_eq!(
        meets
            .iter()
            .map(|meet| (meet.name.as_str(), meet.date.as_str()))
            .collect::<Vec<_>>(),
        vec![("Saint Mary's Invitational", "2026-01-01")]
    );
    assert!(meets[0]
        .source_urls
        .contains(&format!("{}/links/knownid", crate::wayzata::BASE)));
    assert!(harness
        .raw()?
        .iter()
        .any(|row| row["name"] == "Saint Mary's Invitational" && row["slug"] == "knownid"));
    Ok(())
}
