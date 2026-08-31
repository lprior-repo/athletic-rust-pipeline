use crate::alpha_merge::{dedup_athletes, from_ranking_record, merge_athlete};
use crate::alpha_model::{RankingRecord, SourceAthlete, SourceResult};
use std::collections::BTreeMap;

fn result(event: &str, id: Option<u64>, url: Option<&str>) -> SourceResult {
    SourceResult {
        event: event.to_owned(),
        mark: "10.55".to_owned(),
        result_id: id,
        result_url: url.map(str::to_owned),
        ..Default::default()
    }
}

#[test]
fn merge_athlete_preserves_distinct_results() {
    let mut map = BTreeMap::new();
    let mut first = SourceAthlete {
        athlete_id: 12345,
        athlete_name: "John Doe".to_owned(),
        state: "CA".to_owned(),
        ..Default::default()
    };
    first.results.push(result("100m", Some(100), None));
    let mut second = SourceAthlete {
        athlete_id: 12345,
        ..Default::default()
    };
    second.results.push(result("200m", Some(200), None));
    merge_athlete(&mut map, first);
    merge_athlete(&mut map, second);
    assert_eq!(map[&12345].results.len(), 2);
}

#[test]
fn merge_athlete_deduplicates_identical_results() {
    let mut map = BTreeMap::new();
    let mut first = SourceAthlete { athlete_id: 12345, ..Default::default() };
    first.results.push(result("100m", Some(999), None));
    let mut second = SourceAthlete { athlete_id: 12345, ..Default::default() };
    second.results.push(result("100m", Some(999), None));
    merge_athlete(&mut map, first);
    merge_athlete(&mut map, second);
    assert_eq!(map[&12345].results.len(), 1);
}

#[test]
fn merge_athlete_records_identity_conflicts() {
    let mut map = BTreeMap::new();
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        athlete_name: "John Doe".to_owned(),
        ..Default::default()
    });
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        athlete_name: "Johnny Doe".to_owned(),
        ..Default::default()
    });
    assert!(map[&12345].exception_notes.iter().any(|note| note.contains("athlete_name conflict")));
}

#[test]
fn merge_athlete_fills_empty_identity() {
    let mut map = BTreeMap::new();
    merge_athlete(&mut map, SourceAthlete { athlete_id: 12345, ..Default::default() });
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        athlete_name: "John Doe".to_owned(),
        school: "Lincoln".to_owned(),
        state: "CA".to_owned(),
        city: "Los Angeles".to_owned(),
        ..Default::default()
    });
    let athlete = &map[&12345];
    assert_eq!(athlete.athlete_name, "John Doe");
    assert_eq!(athlete.school, "Lincoln");
    assert_eq!(athlete.city, "Los Angeles");
}

#[test]
fn zero_id_merge_returns_exception_only() {
    let mut map = BTreeMap::new();
    let result = merge_athlete(&mut map, SourceAthlete {
        athlete_id: 0,
        athlete_name: "Jane Doe".to_owned(),
        ..Default::default()
    });
    assert_eq!(result.athlete_id, 0);
    assert!(result.exception_notes.iter().any(|note| note.contains("athlete_id")));
    assert!(map.is_empty());
}

#[test]
fn dedup_athletes_separates_missing_ids() {
    let (keyed, exceptions) = dedup_athletes(vec![
        SourceAthlete { athlete_id: 1, athlete_name: "Alice".to_owned(), ..Default::default() },
        SourceAthlete { athlete_id: 0, athlete_name: "Unknown".to_owned(), ..Default::default() },
    ]);
    assert_eq!(keyed.len(), 1);
    assert_eq!(exceptions.len(), 1);
    assert_eq!(exceptions[0].athlete_id, 0);
}

#[test]
fn from_ranking_record_preserves_fields_without_fabricating_result_url() {
    let record = RankingRecord {
        athlete_id: 99999,
        athlete_name: "John Doe".to_owned(),
        grade_id: 11,
        team_name: "Lincoln High".to_owned(),
        state: "CA".to_owned(),
        meet_id: 555,
        meet_name: "State Finals".to_owned(),
        result_id: Some(888),
        event_short: "100m".to_owned(),
        measure: "10.55".to_owned(),
        result_date: "2026-05-01".to_owned(),
        season_id: 2026,
        wind: Some("+1.2".to_owned()),
    };
    let result = from_ranking_record(&record, "https://athletic.net/athlete/99999")
        .expect("approved profile");
    assert_eq!(result.result_id, Some(888));
    assert_eq!(result.event, "100m");
    assert_eq!(result.mark, "10.55");
    assert_eq!(result.meet_name, "State Finals");
    assert_eq!(result.source_url, "https://athletic.net/athlete/99999");
    assert_eq!(result.result_url, None);
}

#[test]
fn from_ranking_record_rejects_wrong_profile() {
    let record = RankingRecord {
        athlete_id: 1,
        athlete_name: "Test".to_owned(),
        grade_id: 12,
        team_name: "High".to_owned(),
        state: "TX".to_owned(),
        meet_id: 0,
        meet_name: "Meet".to_owned(),
        result_id: None,
        event_short: "200m".to_owned(),
        measure: "21.00".to_owned(),
        result_date: "2026-06-01".to_owned(),
        season_id: 2026,
        wind: None,
    };
    assert!(from_ranking_record(&record, "https://example.com/athlete/1").is_err());
}
