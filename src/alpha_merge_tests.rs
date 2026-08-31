/// Tests for merge_athlete: identity, dedup, conflicts, zero ID.
use crate::alpha_merge::{merge_athlete, from_ranking_record, dedup_athletes};
use crate::alpha_normalize::{ResultRecord, SourceAthlete};
use crate::alpha_model::RankingRecord;
use std::collections::BTreeMap;

#[test]
fn merge_athlete_two_events_one_athlete() {
    let mut map = BTreeMap::new();

    let mut a1 = SourceAthlete {
        athlete_id: 12345,
        first_name: "John".to_owned(),
        last_name: "Doe".to_owned(),
        state: "CA".to_owned(),
        ..Default::default()
    };
    a1.results.push(ResultRecord {
        event: "100m".to_owned(),
        mark: "10.55".to_owned(),
        result_url: "https://athletic.net/result/100".to_owned(),
        ..Default::default()
    });

    let mut a2 = SourceAthlete {
        athlete_id: 12345,
        ..Default::default()
    };
    a2.results.push(ResultRecord {
        event: "200m".to_owned(),
        mark: "21.30".to_owned(),
        result_url: "https://athletic.net/result/200".to_owned(),
        ..Default::default()
    });

    merge_athlete(&mut map, a1);
    let result = merge_athlete(&mut map, a2);
    assert_eq!(result.athlete_id, 12345);
    let athlete = &map[&12345];
    assert_eq!(athlete.results.len(), 2);
    assert!(athlete.results.iter().any(|r| r.event == "100m"));
    assert!(athlete.results.iter().any(|r| r.event == "200m"));
}

#[test]
fn merge_athlete_dedup_by_result_id() {
    let mut map = BTreeMap::new();
    let mut a1 = SourceAthlete { athlete_id: 12345, ..Default::default() };
    a1.results.push(ResultRecord {
        result_id: 999, event: "100m".to_owned(), mark: "10.55".to_owned(), ..Default::default()
    });
    let mut a2 = SourceAthlete { athlete_id: 12345, ..Default::default() };
    a2.results.push(ResultRecord {
        result_id: 999, event: "100m".to_owned(), mark: "10.55".to_owned(), ..Default::default()
    });
    merge_athlete(&mut map, a1);
    merge_athlete(&mut map, a2);
    assert_eq!(map[&12345].results.iter().filter(|r| r.result_id == 999).count(), 1);
}

#[test]
fn merge_athlete_identity_conflict_creates_exception() {
    let mut map = BTreeMap::new();
    let a1 = SourceAthlete { athlete_id: 12345, first_name: "John".to_owned(), last_name: "Doe".to_owned(), ..Default::default() };
    let a2 = SourceAthlete { athlete_id: 12345, first_name: "Johnny".to_owned(), last_name: "Doe".to_owned(), ..Default::default() };
    merge_athlete(&mut map, a1);
    merge_athlete(&mut map, a2);
    assert!(map[&12345].exception_notes.iter().any(|n| n.contains("first_name conflict")));
}

#[test]
fn merge_athlete_fills_empty_identity() {
    let mut map = BTreeMap::new();
    let a1 = SourceAthlete { athlete_id: 12345, ..Default::default() };
    let a2 = SourceAthlete {
        athlete_id: 12345,
        first_name: "John".to_owned(),
        last_name: "Doe".to_owned(),
        school: "Lincoln".to_owned(),
        state: "CA".to_owned(),
        city: "Los Angeles".to_owned(),
        ..Default::default()
    };
    merge_athlete(&mut map, a1);
    merge_athlete(&mut map, a2);
    let a = &map[&12345];
    assert_eq!(a.first_name, "John");
    assert_eq!(a.last_name, "Doe");
    assert_eq!(a.school, "Lincoln");
    assert_eq!(a.state, "CA");
    assert_eq!(a.city, "Los Angeles");
}

#[test]
fn merge_athlete_zero_id_returns_exception_only() {
    let mut map = BTreeMap::new();
    let a = SourceAthlete { athlete_id: 0, first_name: "Jane".to_owned(), ..Default::default() };
    let result = merge_athlete(&mut map, a);
    assert_eq!(result.athlete_id, 0);
    assert!(result.exception_notes.iter().any(|n| n.contains("athlete_id missing or zero")));
    // Zero-ID athletes should NOT be in the keyed map
    assert_eq!(map.len(), 0);
}

#[test]
fn dedup_athletes_merges_duplicates() {
    let mut a1 = SourceAthlete {
        athlete_id: 12345,
        first_name: "John".to_owned(),
        school: "Lincoln".to_owned(),
        ..Default::default()
    };
    a1.results.push(ResultRecord { event: "100m".to_owned(), mark: "10.55".to_owned(), ..Default::default() });
    let mut a2 = SourceAthlete {
        athlete_id: 12345,
        last_name: "Doe".to_owned(),
        school: "Jefferson".to_owned(),
        ..Default::default()
    };
    a2.results.push(ResultRecord { event: "200m".to_owned(), mark: "21.30".to_owned(), ..Default::default() });
    let (deduped, exception_only) = dedup_athletes(vec![a1, a2]);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].first_name, "John");
    assert_eq!(deduped[0].last_name, "Doe");
    assert_eq!(deduped[0].results.len(), 2);
    assert_eq!(exception_only.len(), 0);
}

#[test]
fn dedup_athletes_keeps_distinct_ids() {
    let a1 = SourceAthlete { athlete_id: 1, first_name: "Alice".to_owned(), ..Default::default() };
    let a2 = SourceAthlete { athlete_id: 2, first_name: "Bob".to_owned(), ..Default::default() };
    let (deduped, exception_only) = dedup_athletes(vec![a1, a2]);
    assert_eq!(deduped.len(), 2);
    assert_eq!(exception_only.len(), 0);
}

#[test]
fn dedup_athletes_separates_zero_id() {
    let a1 = SourceAthlete { athlete_id: 12345, first_name: "John".to_owned(), ..Default::default() };
    let a2 = SourceAthlete { athlete_id: 0, first_name: "Jane".to_owned(), ..Default::default() };
    let (deduped, exception_only) = dedup_athletes(vec![a1, a2]);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].first_name, "John");
    assert_eq!(exception_only.len(), 1);
    assert_eq!(exception_only[0].first_name, "Jane");
    assert!(exception_only[0].exception_notes.iter().any(|n| n.contains("athlete_id missing or zero")));
}

#[test]
fn from_ranking_record_preserves_all_fields() {
    let rec = RankingRecord {
        athlete_id: 99999, athlete_name: "John Doe".to_owned(), grade_id: 11,
        team_name: "Lincoln High".to_owned(), state: "CA".to_owned(), meet_id: 555,
        meet_name: "State Finals".to_owned(), result_id: Some(888), event_short: "100m".to_owned(),
        measure: "10.55".to_owned(), result_date: "2026-05-01".to_owned(), season_id: 2026,
        wind: Some("+1.2".to_owned()),
    };
    let rr = from_ranking_record(&rec, "https://athletic.net/athlete/99999");
    assert_eq!(rr.result_id, 888);
    assert_eq!(rr.event, "100m");
    assert_eq!(rr.mark, "10.55");
    assert_eq!(rr.season, "2026");
    assert_eq!(rr.date, "2026-05-01");
    assert_eq!(rr.meet_name, "State Finals");
    assert_eq!(rr.wind, Some("+1.2".to_owned()));
    assert_eq!(rr.source_url, "https://athletic.net/athlete/99999");
    assert_eq!(rr.result_url, "https://athletic.net/result/888");
}

#[test]
fn from_ranking_record_no_result_id() {
    let rec = RankingRecord {
        athlete_id: 1, athlete_name: "Test".to_owned(), grade_id: 12,
        team_name: "High".to_owned(), state: "TX".to_owned(), meet_id: 0,
        meet_name: "Meet".to_owned(), result_id: None, event_short: "200m".to_owned(),
        measure: "21.00".to_owned(), result_date: "2026-06-01".to_owned(), season_id: 2026, wind: None,
    };
    let rr = from_ranking_record(&rec, "");
    assert_eq!(rr.result_id, 0);
    assert_eq!(rr.event, "200m");
    assert_eq!(rr.wind, None);
    assert_eq!(rr.result_url, "");
}
