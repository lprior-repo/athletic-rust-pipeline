/// Additional tests for RankingRecord conversion and URL merging.
use crate::alpha_merge::{from_model_source_result, merge_athlete, dedup_athletes};
use crate::alpha_normalize::{ResultRecord, SourceAthlete};
use crate::alpha_url::{validate_result_url, validate_source_url};
use crate::alpha_normalize::parse_location;
use crate::alpha_model::SourceResult as ModelSourceResult;

#[test]
fn test_from_model_source_result() {
    let sr = ModelSourceResult {
        result_id: 777,
        event_short: "400m".to_owned(),
        measure: "48.50".to_owned(),
        result_date: "2026-07-01".to_owned(),
        season_id: 2027,
    };
    let rr = from_model_source_result(&sr);
    assert_eq!(rr.result_id, 777);
    assert_eq!(rr.event, "400m");
    assert_eq!(rr.mark, "48.50");
    assert_eq!(rr.season, "2027");
    assert_eq!(rr.date, "2026-07-01");
    assert_eq!(rr.result_url, "");
}

#[test]
fn merge_athlete_profile_urls_merged() {
    let mut map = std::collections::BTreeMap::new();
    let a1 = SourceAthlete {
        athlete_id: 12345,
        profile_urls: vec!["https://athletic.net/athlete/12345".to_owned()],
        ..Default::default()
    };
    let a2 = SourceAthlete {
        athlete_id: 12345,
        profile_urls: vec![
            "https://athletic.net/athlete/12345".to_owned(),
            "https://www.athletic.net/athlete/12345".to_owned(),
        ],
        ..Default::default()
    };
    merge_athlete(&mut map, a1);
    merge_athlete(&mut map, a2);
    assert_eq!(map[&12345].profile_urls.len(), 2);
}

#[test]
fn merge_athlete_source_urls_merged() {
    let mut map = std::collections::BTreeMap::new();
    let a1 = SourceAthlete {
        athlete_id: 12345,
        source_urls: vec!["https://athletic.net/sheet/1".to_owned()],
        ..Default::default()
    };
    let a2 = SourceAthlete {
        athlete_id: 12345,
        source_urls: vec![
            "https://athletic.net/sheet/1".to_owned(),
            "https://athletic.net/sheet/2".to_owned(),
        ],
        ..Default::default()
    };
    merge_athlete(&mut map, a1);
    merge_athlete(&mut map, a2);
    assert_eq!(map[&12345].source_urls.len(), 2);
}

#[test]
fn merge_athlete_result_url_not_in_result_urls() {
    let mut a = SourceAthlete {
        athlete_id: 12345,
        profile_urls: vec!["https://athletic.net/athlete/12345".to_owned()],
        ..Default::default()
    };
    a.results.push(ResultRecord {
        result_url: "https://athletic.net/athlete/12345".to_owned(),
        ..Default::default()
    });
    a.results.push(ResultRecord {
        result_url: "https://athletic.net/result/100".to_owned(),
        ..Default::default()
    });
    let (deduped, exception_only) = dedup_athletes(vec![a]);
    assert_eq!(deduped[0].profile_urls.len(), 1);
    assert_eq!(exception_only.len(), 0);
}

#[test]
fn zero_id_merge_returns_exception_only() {
    let mut map = std::collections::BTreeMap::new();
    let a = SourceAthlete {
        athlete_id: 0,
        first_name: "Jane".to_owned(),
        last_name: "Doe".to_owned(),
        state: "CA".to_owned(),
        city: "LA".to_owned(),
        profile_urls: vec!["https://athletic.net/athlete/0".to_owned()],
        source_urls: vec!["https://athletic.net/sheet/1".to_owned()],
        results: vec![ResultRecord {
            event: "100m".to_owned(),
            mark: "11.00".to_owned(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = merge_athlete(&mut map, a);
    assert_eq!(result.athlete_id, 0);
    assert!(result.exception_notes.iter().any(|n| n.contains("athlete_id missing or zero")));
    assert_eq!(result.first_name, "Jane");
    assert_eq!(result.last_name, "Doe");
    assert_eq!(result.state, "CA");
    assert_eq!(result.city, "LA");
    assert_eq!(result.profile_urls.len(), 1);
    assert_eq!(result.source_urls.len(), 1);
    assert_eq!(result.results.len(), 1);
    // Zero-ID athlete should NOT be in the keyed map
    assert_eq!(map.len(), 0);
}

#[test]
fn validate_result_url_rejects_non_numeric_id() {
    assert_eq!(validate_result_url("https://athletic.net/result/abc"), None);
    assert_eq!(validate_result_url("https://athletic.net/result/123xyz"), None);
    assert_eq!(validate_result_url("https://athletic.net/result/"), None);
    let canonical = validate_result_url("https://www.athletic.net/result/789");
    assert_eq!(canonical, Some("https://athletic.net/result/789".to_owned()));
}

#[test]
fn validate_result_url_rejects_non_default_port() {
    assert_eq!(validate_result_url("https://athletic.net:8080/result/789"), None);
    assert_eq!(validate_result_url("https://athletic.net:443/result/789"), Some("https://athletic.net/result/789".to_owned()));
}

#[test]
fn validate_source_url_rejects_query_and_fragment() {
    assert_eq!(validate_source_url("https://athletic.net/sheet/1?foo=bar"), None);
    assert_eq!(validate_source_url("https://athletic.net/sheet/1#section"), None);
    assert_eq!(validate_source_url("https://athletic.net/sheet/1?a=1&b=2#top"), None);
    assert_eq!(validate_source_url("https://athletic.net/sheet/1"), Some("https://athletic.net/sheet/1".to_owned()));
}

#[test]
fn validate_source_url_rejects_non_default_port() {
    assert_eq!(validate_source_url("https://athletic.net:8080/sheet/1"), None);
    assert_eq!(validate_source_url("https://athletic.net:443/sheet/1"), Some("https://athletic.net/sheet/1".to_owned()));
}

#[test]
fn parse_location_rejects_digit_starting_address() {
    assert_eq!(parse_location("123 Main St"), None);
    assert_eq!(parse_location("456 Oak Ave"), None);
    assert_eq!(parse_location("Los Angeles, CA"), Some(("Los Angeles".to_owned(), "CA".to_owned())));
    assert_eq!(parse_location("New York"), Some(("New York".to_owned(), String::new())));
    assert_eq!(parse_location("Springfield"), Some(("Springfield".to_owned(), String::new())));
}

#[test]
fn canonical_state_rejects_dc() {
    use crate::alpha_url::canonical_state;
    assert_eq!(canonical_state("DC"), None);
    assert_eq!(canonical_state("district of columbia"), None);
    assert_eq!(canonical_state("CA"), Some("CA".to_owned()));
    assert_eq!(canonical_state("California"), Some("CA".to_owned()));
}

#[test]
fn distinct_zero_id_exceptions_not_collapsed() {
    let a1 = SourceAthlete { athlete_id: 0, first_name: "Jane".to_owned(), ..Default::default() };
    let a2 = SourceAthlete { athlete_id: 0, first_name: "John".to_owned(), ..Default::default() };
    // Zero-ID athletes should NOT go into the keyed map
    let mut map = std::collections::BTreeMap::new();
    let r1 = merge_athlete(&mut map, a1);
    let r2 = merge_athlete(&mut map, a2);
    assert_eq!(r1.athlete_id, 0);
    assert_eq!(r2.athlete_id, 0);
    assert_ne!(r1.first_name, r2.first_name);
    // Both should be returned outside the keyed map
    assert_eq!(map.len(), 0);
    let (deduped, exception_only) = dedup_athletes(vec![
        SourceAthlete { athlete_id: 12345, first_name: "Bob".to_owned(), ..Default::default() },
        r1,
        r2,
    ]);
    assert_eq!(deduped.len(), 1);
    assert_eq!(exception_only.len(), 2);
}
