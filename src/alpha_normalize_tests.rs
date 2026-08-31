/// Focused tests for normalization: field extraction, URL validation, city parsing.
use crate::alpha_normalize::{
    canonical_state, normalize_record, normalize_whitespace, parse_location, SourceRecord,
};
use std::collections::BTreeMap;

#[test]
fn canonical_state_uppercases() {
    assert_eq!(canonical_state("ca").unwrap(), "CA");
    assert_eq!(canonical_state("  ca  ").unwrap(), "CA");
    assert_eq!(canonical_state("ny").unwrap(), "NY");
    assert!(canonical_state("zz").is_none());
}

#[test]
fn canonical_state_maps_full_names() {
    assert_eq!(canonical_state("california").unwrap(), "CA");
    assert_eq!(canonical_state("California").unwrap(), "CA");
    assert_eq!(canonical_state("texas").unwrap(), "TX");
    assert!(canonical_state("somewhere").is_none());
}

#[test]
fn normalize_whitespace_collapses() {
    assert_eq!(normalize_whitespace("  hello    world  "), "hello world");
    assert_eq!(normalize_whitespace("no-spaces"), "no-spaces");
    assert_eq!(normalize_whitespace("   "), "");
}

#[test]
fn normalize_record_extract_fields() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert("first_name".to_owned(), "  John  ".to_owned());
    fields.insert("last_name".to_owned(), "Doe".to_owned());
    fields.insert("school".to_owned(), "  Lincoln High  ".to_owned());
    fields.insert("state".to_owned(), "ca".to_owned());
    fields.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/12345".to_owned(),
    );

    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "sheet1".to_owned(),
        excel_row: 1,
        fields,
    };

    let athlete = normalize_record(&record);
    assert_eq!(athlete.first_name, "John");
    assert_eq!(athlete.last_name, "Doe");
    assert_eq!(athlete.school, "Lincoln High");
    assert_eq!(athlete.state, "CA");
    assert_eq!(athlete.profile_urls, vec!["https://athletic.net/athlete/12345"]);
}

#[test]
fn normalize_record_keeps_separate_names() {
    let mut fields = BTreeMap::new();
    fields.insert("first_name".to_owned(), "Jane".to_owned());
    fields.insert("last_name".to_owned(), "Smith".to_owned());

    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };

    let athlete = normalize_record(&record);
    assert_eq!(athlete.first_name, "Jane");
    assert_eq!(athlete.last_name, "Smith");
}

#[test]
fn normalize_record_rejects_invalid_profile_url() {
    let mut fields = BTreeMap::new();
    fields.insert(
        "profile_url".to_owned(),
        "http://athletic.net/athlete/123".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert!(athlete.profile_urls.is_empty());
}

#[test]
fn normalize_record_multiple_profile_urls() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "1".to_owned());
    fields.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/1;https://www.athletic.net/athlete/1".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(athlete.profile_urls.len(), 1);
}

#[test]
fn normalize_record_result_urls_excluded_from_profile() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/12345".to_owned(),
    );
    fields.insert(
        "result_urls".to_owned(),
        "https://athletic.net/result/100;https://athletic.net/result/200".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert!(!athlete.profile_urls.contains(&"https://athletic.net/result/100".to_owned()));
    assert_eq!(athlete.profile_urls.len(), 1);
    assert_eq!(athlete.results.len(), 2);
    assert!(athlete.results.iter().any(|r| r.result_url == Some("https://athletic.net/result/100".to_owned())));
    assert!(athlete.results.iter().any(|r| r.result_url == Some("https://athletic.net/result/200".to_owned())));
}

#[test]
fn normalize_record_parse_marks() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert("marks".to_owned(), "100m|10.55|2026-27|2026-05-01|Invitational|+1.2".to_owned());
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(athlete.results.len(), 1);
    assert_eq!(athlete.results[0].event, "100m");
    assert_eq!(athlete.results[0].mark, "10.55");
    assert_eq!(athlete.results[0].season, "2026-27");
    assert_eq!(athlete.results[0].date, "2026-05-01");
    assert_eq!(athlete.results[0].meet_name, "Invitational");
    assert_eq!(athlete.results[0].wind, Some("+1.2".to_owned()));
}

#[test]
fn normalize_record_attaches_result_ids() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert("marks".to_owned(), "100m|10.55|2026-27|2026-05-01|Invitational".to_owned());
    fields.insert("result_ids".to_owned(), "999".to_owned());
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(athlete.results[0].result_id, Some(999));
}

#[test]
fn parse_location_city_state() {
    let (city, state) = parse_location("Los Angeles, CA").unwrap();
    assert_eq!(city, "Los Angeles");
    assert_eq!(state, "CA");
}

#[test]
fn parse_location_bare_city() {
    let (city, state) = parse_location("Chicago").unwrap();
    assert_eq!(city, "Chicago");
    assert_eq!(state, "");
}

#[test]
fn parse_location_rejects_digits() {
    assert!(parse_location("555-1234").is_none());
    assert!(parse_location("12345").is_none());
}

#[test]
fn normalize_record_unknown_state_exception() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert("state".to_owned(), "ZZ".to_owned());
    let record = SourceRecord {
        source_key: "bad".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(athlete.state, "");
    assert!(!athlete.exception_notes.is_empty());
}

#[test]
fn normalize_record_source_url_multiple() {
    let mut fields = BTreeMap::new();
    fields.insert(
        "source_url".to_owned(),
        "https://athletic.net/sheet/1;https://athletic.net/sheet/2".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(athlete.source_urls.len(), 2);
}

#[test]
fn normalize_record_invalid_mark_adds_exception_note() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert("marks".to_owned(), "bogus_event|10.55|2026-27|2026-05-01".to_owned());
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert!(athlete.exception_notes.iter().any(|n| n.contains("invalid mark")));
    assert_eq!(athlete.results.len(), 0);
}

#[test]
fn normalize_record_invalid_city_adds_exception_note() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert("city".to_owned(), "123 Main St".to_owned());
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert!(athlete.exception_notes.iter().any(|n| n.contains("invalid city")));
    assert_eq!(athlete.city, "");
}

#[test]
fn normalize_record_profile_url_id_mismatch_adds_exception_note() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "11111".to_owned());
    fields.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/99999".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert!(athlete.exception_notes.iter().any(|n| n.contains("profile URL athlete ID conflicts")));
}

#[test]
fn normalize_record_source_url_canonicalizes_www() {
    let mut fields = BTreeMap::new();
    fields.insert(
        "source_url".to_owned(),
        "https://www.athletic.net/sheet/1".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(
        athlete.source_urls,
        vec!["https://athletic.net/sheet/1".to_owned()]
    );
}
