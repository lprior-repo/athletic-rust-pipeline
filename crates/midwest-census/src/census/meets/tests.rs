//! What the meet census must guarantee: rows keyed by the provider's own id, a reproducible
//! selection order, and a season that ends when a page repeats its predecessor.
use super::*;

fn meet(meet_id: &str, date: Option<&str>) -> MeetRef {
    MeetRef {
        meet_id: meet_id.to_string(),
        name: "Beaver Eastern Invite".to_string(),
        date: date.map(str::to_string),
        venue: "Beaver, OH".to_string(),
        results_url: format!(
            "https://oh.milesplit.com/meets/{meet_id}-beaver-eastern-invite-2026/results"
        ),
    }
}

#[test]
fn a_row_is_keyed_by_the_providers_own_meet_id() {
    let row = source_meet_row(
        &meet("770621", Some("2026-09-19")),
        UsJurisdiction::Ohio,
        Season::CrossCountry,
        2026,
        "2026-09-22",
    );
    assert_eq!(row.id, "milesplit:770621");
    assert_eq!(row.source_meet_id, "770621");
    assert_eq!(row.season, "cc");
    assert_eq!(row.observed_on, "2026-09-22");
}

/// A results-index row publishes the meet's day only when it publishes both a month and a day.
/// A meet with a month and no day stays day-less rather than acquiring one.
#[test]
fn a_meet_without_a_published_day_stays_day_less() {
    let row = source_meet_row(
        &meet("770622", None),
        UsJurisdiction::Ohio,
        Season::Outdoor,
        2026,
        "2026-09-22",
    );
    assert_eq!(row.date, None);
    assert_eq!(row.season, "outdoor");
}

fn row(fragment: &str, jurisdiction: UsJurisdiction, meet_id: &str) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(SOURCE, meet_id),
        source: SOURCE.to_string(),
        source_meet_id: meet_id.to_string(),
        jurisdiction,
        season: "cc".to_string(),
        year: 2026,
        name: fragment.to_string(),
        date: None,
        venue: String::new(),
        results_url: format!(
            "https://{}.milesplit.com/meets/{meet_id}/results",
            jurisdiction.code().to_ascii_lowercase()
        ),
        observed_on: "2026-09-22".to_string(),
    }
}

/// The selection is ordered by the provider's own meet id, so two runs read the same meets in
/// the same sequence whatever order the store returned them in.
#[test]
fn meets_are_selected_in_a_reproducible_order() {
    let rows = vec![
        row("c", UsJurisdiction::Ohio, "900"),
        row("a", UsJurisdiction::Ohio, "100"),
        row("b", UsJurisdiction::Michigan, "100"),
    ];
    let selected = select_meets(
        rows,
        &[UsJurisdiction::Ohio, UsJurisdiction::Michigan],
        None,
    );
    let order: Vec<(&str, &str)> = selected
        .iter()
        .map(|meet| (meet.jurisdiction.code(), meet.source_meet_id.as_str()))
        .collect();
    assert_eq!(order, [("MI", "100"), ("OH", "100"), ("OH", "900")]);
}

/// A limit applies per state, not across the selection: one state's meets cannot fill a run
/// that names two states.
#[test]
fn a_limit_is_applied_per_state() {
    let rows = vec![
        row("a", UsJurisdiction::Ohio, "1"),
        row("b", UsJurisdiction::Ohio, "2"),
        row("c", UsJurisdiction::Ohio, "3"),
        row("d", UsJurisdiction::Michigan, "9"),
    ];
    let selected = select_meets(
        rows,
        &[UsJurisdiction::Ohio, UsJurisdiction::Michigan],
        Some(2),
    );
    let ohio = selected
        .iter()
        .filter(|meet| meet.jurisdiction == UsJurisdiction::Ohio)
        .count();
    let michigan = selected
        .iter()
        .filter(|meet| meet.jurisdiction == UsJurisdiction::Michigan)
        .count();
    assert_eq!((ohio, michigan), (2, 1));
}

/// The wrap signal: the same page twice ends the season, a different page does not, and the
/// first page can never be a repeat.
#[test]
fn a_page_that_repeats_its_predecessor_ends_the_season() {
    let first = vec!["1".to_string(), "2".to_string()];
    assert!(!repeats_previous(None, &first));
    assert!(repeats_previous(Some(&first), &first));
    assert!(!repeats_previous(
        Some(&first),
        &["3".to_string(), "4".to_string()]
    ));
    // An empty page is only a repeat of another empty page: a meet index with nothing in it is
    // a state to report, not a wrap to stop on.
    let empty: Vec<String> = Vec::new();
    assert!(!repeats_previous(None, &empty));
    assert!(repeats_previous(Some(&empty), &empty));
}

/// A state with no stored meets contributes nothing rather than an invented row.
#[test]
fn a_state_with_no_stored_meets_selects_nothing() {
    let rows = vec![row("a", UsJurisdiction::Ohio, "1")];
    let selected = select_meets(rows, &[UsJurisdiction::Kansas], None);
    assert!(selected.is_empty());
}

/// The journal phase names the state, season and year *and* a version: a parser change that
/// alters what an already-journaled page yields bumps the version, so those pages are read
/// again instead of being skipped as done.
#[test]
fn the_journal_phase_carries_the_version_that_forces_a_re_read() {
    assert_eq!(
        meets_phase(UsJurisdiction::Ohio, Season::CrossCountry, 2026),
        "milesplit_meet_index_oh_cc_2026_v1"
    );
}
