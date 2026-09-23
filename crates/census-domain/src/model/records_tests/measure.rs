//! What the measurement rows say: one row per subject, one snapshot per pass, one condition per host
//! and kind.

use super::super::*;

#[test]
fn coverage_rows_are_one_per_subject_and_name_their_metrics() {
    let row = CoverageRow::new(CoverageScope::Jurisdiction, "WI")
        .with("schools", 12)
        .with("cohort_athletes", 34);
    assert_eq!(row.id, "jurisdiction:WI");
    assert_eq!(row.metrics.get("cohort_athletes"), Some(&34));

    let source = CoverageRow::new(CoverageScope::Source, "milesplit_athlete").with("identities", 7);
    assert_eq!(source.id, "source:milesplit_athlete");
    assert_ne!(row.id, source.id);
}

#[test]
fn a_snapshot_is_keyed_by_the_pass_that_finished() {
    let snapshot = CollectionSnapshot::new("derive", "2026-09-22")
        .with_observations("athletes", 10)
        .with_observations("schools", 4);
    assert_eq!(snapshot.id, "derive:2026-09-22");
    assert_eq!(snapshot.observations.get("schools"), Some(&4));
    assert_ne!(
        snapshot.id,
        CollectionSnapshot::new("derive", "2026-09-23").id,
        "each pass gets its own snapshot row"
    );
}

#[test]
fn an_access_condition_is_keyed_by_kind_and_host() {
    let condition = SourceAccessCondition::new(
        "milesplit",
        "ct.milesplit.com",
        AccessBlockKind::Forbidden,
        403,
        "2026-09-22T07:11:19Z",
        "GET /teams/1/roster",
    );
    assert_eq!(condition.id, "forbidden:ct.milesplit.com");
    assert_eq!(condition.kind.slug(), "forbidden");
    assert_ne!(
        condition.id,
        SourceAccessCondition::new(
            "milesplit",
            "ct.milesplit.com",
            AccessBlockKind::RateLimited,
            429,
            "2026-09-22T07:11:19Z",
            "GET /teams/1/roster",
        )
        .id,
        "a rate limit and a refusal are different findings about one host"
    );
    assert_eq!(
        AccessBlockKind::RobotsDisallowed.slug(),
        "robots_disallowed"
    );
    assert_eq!(AccessBlockKind::Unavailable.slug(), "unavailable");
    assert_eq!(
        AccessBlockKind::BrowserUnavailable.slug(),
        "browser_unavailable"
    );
    assert_eq!(AccessBlockKind::HumanRequired.slug(), "human_required");
}

#[test]
fn a_condition_blocks_until_its_cooldown_passes_and_forever_without_one() {
    let open = SourceAccessCondition::new(
        "wiaa",
        "www.wiaa.org",
        AccessBlockKind::Forbidden,
        403,
        "2026-09-22T00:00:00Z",
        "GET /results",
    )
    .with_cooldown_until(Some("2026-09-22T06:00:00Z".to_string()));
    assert!(open.is_blocking("2026-09-22T05:59:59Z"));
    assert!(!open.is_blocking("2026-09-22T06:00:00Z"));
    assert!(!open.is_blocking("2026-09-23T00:00:00Z"));

    let endless = open.clone().with_cooldown_until(None);
    assert!(
        endless.is_blocking("2099-01-01T00:00:00Z"),
        "a condition with no cooldown is the operator's problem until they clear it"
    );

    let timed = open.clone().with_retry_after(Some(120));
    assert_eq!(timed.retry_after_seconds, Some(120));
    assert_eq!(
        timed.id, open.id,
        "the published delay changes the row, not its key"
    );
}
