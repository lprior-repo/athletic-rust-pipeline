/// Additional tests for RankingRecord conversion and URL merging.
use crate::alpha_merge::{from_model_source_result, merge_athlete, dedup_athletes};
use crate::alpha_normalize::{ResultRecord, SourceAthlete};
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
    let rr = from_model_source_result(&sr, "https://athletic.net/athlete/123");
    assert_eq!(rr.result_id, 777);
    assert_eq!(rr.event, "400m");
    assert_eq!(rr.mark, "48.50");
    assert_eq!(rr.season, "2027");
    assert_eq!(rr.date, "2026-07-01");
    assert_eq!(rr.result_url, "https://athletic.net/result/777");
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
    let deduped = dedup_athletes(vec![a]);
    assert_eq!(deduped[0].profile_urls.len(), 1);
}
