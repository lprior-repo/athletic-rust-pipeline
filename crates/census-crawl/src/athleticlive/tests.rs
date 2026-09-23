use super::*;
use census_domain::model::{CompetitionLevel, SourceIdentity, SourceNamespace};

const HARVEST: &str = include_str!("../../tests/fixtures/athleticlive/meets-sample.csv");

#[test]
fn parses_harvest_rows_and_keeps_only_known_states() {
    let rows = parse_meets_csv(HARVEST).expect("fixture parses");
    assert!(rows.len() >= 4, "fixture should carry several rows");
    assert!(
        rows.iter().all(|r| !r.name.is_empty()),
        "every kept row has a name"
    );
    assert!(rows
        .iter()
        .any(|r| r.state_code == UsJurisdiction::Illinois));
    assert!(
        rows.iter().any(|r| r.athleticnet_meet_id.is_some()),
        "the harvest carries Athletic.net meet ids"
    );
}

#[test]
fn quoted_fields_do_not_break_column_alignment() {
    let csv = "tenant,athleticlive_meet_id,athleticnet_meet_id,name,city_state,state,start,end,has_results,timer_credit\n\
               palatine,55274,259955,\"4th Annual St. Pius X, Knights Classic\",\"Lombard, IL\",Illinois,2025-08-16T04:00:00Z,,True,\"Timed by <a href=\"\"x\"\">Palatine Pack</a>\"\n";
    let rows = parse_meets_csv(csv).expect("quoted line parses");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "4th Annual St. Pius X, Knights Classic");
    assert_eq!(rows[0].city_state.as_deref(), Some("Lombard, IL"));
    assert_eq!(rows[0].state_code, UsJurisdiction::Illinois);
    assert_eq!(rows[0].athleticnet_meet_id.as_deref(), Some("259955"));
    assert!(rows[0].has_results);
}

#[test]
fn missing_required_column_is_an_error_not_a_panic() {
    let err = parse_meets_csv("tenant,name,state,start\nx,Meet,Illinois,2025-08-16T04:00:00Z\n")
        .expect_err("athleticlive_meet_id is required");
    assert!(err.to_string().contains("athleticlive_meet_id"));
}

#[test]
fn corrupt_years_are_flagged_not_silently_kept() {
    assert!(!implausible_year("2026-05-29"));
    assert!(!implausible_year("2015-01-01"));
    assert!(implausible_year("2222-08-23"));
    assert!(implausible_year("not-a-date"));
    let rows = parse_meets_csv(HARVEST).expect("fixture parses");
    assert!(rows.iter().all(|r| !implausible_year(&r.start)));
}

#[test]
fn level_inference_only_fires_on_explicit_markers() {
    assert_eq!(
        infer_level("WIAA State Championships"),
        CompetitionLevel::State
    );
    assert_eq!(infer_level("D3 Sectional #3"), CompetitionLevel::Sectional);
    assert_eq!(
        infer_level("Big Rivers Conference Meet"),
        CompetitionLevel::Conference
    );
    assert_eq!(
        infer_level("Knights Chicagoland Classic"),
        CompetitionLevel::Invitational
    );
    assert_eq!(infer_level("Weekend Race #4"), CompetitionLevel::Unknown);
}

#[test]
fn tenants_publishing_one_meet_merge_into_one_canonical_meet() {
    let rows = parse_meets_csv(HARVEST).expect("fixture parses");
    let meets = build_meets(&rows, "2026-09-20", "athleticlive_meets_csv");
    let classic = meets
        .iter()
        .find(|m| m.name.contains("Knights Chicagoland"))
        .expect("fixture meet present");
    let timers: Vec<&SourceIdentity> = classic
        .source_identities
        .iter()
        .filter(|i| matches!(i.namespace, SourceNamespace::TimerMeet { .. }))
        .collect();
    assert!(
        timers.len() >= 2,
        "both tenants are recorded on one canonical meet"
    );
    assert_eq!(timers[0].id, "55274");
    let an: Vec<&SourceIdentity> = classic
        .source_identities
        .iter()
        .filter(|i| matches!(i.namespace, SourceNamespace::LegacyAthleticNet { .. }))
        .collect();
    assert_eq!(an.len(), 1, "the Athletic.net meet id is deduplicated");
    assert_eq!(an[0].id, "259955");
    assert_eq!(
        an[0].url.as_deref(),
        Some("https://www.athletic.net/TrackAndField/meet/259955/info")
    );
    assert_eq!(classic.location.as_deref(), Some("Lombard, IL"));
    assert_eq!(classic.date, "2025-08-16");
    assert_eq!(meets.iter().filter(|m| m.id == classic.id).count(), 1);
}
