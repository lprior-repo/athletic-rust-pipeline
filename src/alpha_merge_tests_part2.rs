use crate::alpha_merge::{dedup_athletes, from_model_source_result, merge_athlete};
use crate::alpha_model::{SourceAthlete, SourceResult};
use crate::alpha_normalize::parse_location;
use crate::alpha_url::{validate_result_url, validate_source_url};

#[test]
fn model_source_result_is_preserved() {
    let source = SourceResult {
        result_id: Some(777),
        event: "400m".to_owned(),
        mark: "48.50".to_owned(),
        season: "2026-27".to_owned(),
        date: "2026-07-01".to_owned(),
        meet_name: "Regional".to_owned(),
        wind: Some("+1.0".to_owned()),
        source_url: "https://athletic.net/athlete/123".to_owned(),
        result_url: None,
    };
    assert_eq!(from_model_source_result(&source), source);
}

#[test]
fn merge_athlete_profile_urls_merge_without_duplicates() {
    let mut map = std::collections::BTreeMap::new();
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        profile_urls: vec!["https://athletic.net/athlete/12345".to_owned()],
        ..Default::default()
    });
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        profile_urls: vec![
            "https://athletic.net/athlete/12345".to_owned(),
            "https://athletic.net/athlete/54321".to_owned(),
        ],
        ..Default::default()
    });
    assert_eq!(map[&12345].profile_urls.len(), 2);
}

#[test]
fn merge_athlete_source_urls_preserves_all_sources() {
    let mut map = std::collections::BTreeMap::new();
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        source_urls: vec!["https://athletic.net/sheet/1".to_owned()],
        ..Default::default()
    });
    merge_athlete(&mut map, SourceAthlete {
        athlete_id: 12345,
        source_urls: vec![
            "https://athletic.net/sheet/1".to_owned(),
            "https://athletic.net/sheet/2".to_owned(),
        ],
        ..Default::default()
    });
    assert_eq!(map[&12345].source_urls.len(), 2);
}

#[test]
fn zero_id_records_remain_exception_only_and_distinct() {
    let (keyed, exceptions) = dedup_athletes(vec![
        SourceAthlete { athlete_id: 0, athlete_name: "Jane".to_owned(), ..Default::default() },
        SourceAthlete { athlete_id: 0, athlete_name: "John".to_owned(), ..Default::default() },
    ]);
    assert!(keyed.is_empty());
    assert_eq!(exceptions.len(), 2);
    assert_ne!(exceptions[0].athlete_name, exceptions[1].athlete_name);
}

#[test]
fn validate_result_url_requires_numeric_id_and_default_port() {
    assert!(validate_result_url("https://athletic.net/result/abc").is_none());
    assert!(validate_result_url("https://athletic.net:8080/result/789").is_none());
    assert_eq!(
        validate_result_url("https://www.athletic.net/result/789"),
        Some("https://athletic.net/result/789".to_owned())
    );
}

#[test]
fn validate_source_url_rejects_query_fragment_and_non_default_port() {
    assert!(validate_source_url("https://athletic.net/sheet/1?x=1").is_none());
    assert!(validate_source_url("https://athletic.net/sheet/1#part").is_none());
    assert!(validate_source_url("https://athletic.net:8080/sheet/1").is_none());
    assert_eq!(
        validate_source_url("https://athletic.net/sheet/1"),
        Some("https://athletic.net/sheet/1".to_owned())
    );
}

#[test]
fn parse_location_rejects_private_data_and_preserves_state() {
    assert!(parse_location("123 Main St").is_none());
    assert!(parse_location("alice@example.com").is_none());
    assert_eq!(
        parse_location("Los Angeles, CA"),
        Some(("Los Angeles".to_owned(), "CA".to_owned()))
    );
}
