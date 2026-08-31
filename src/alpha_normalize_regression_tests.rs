use crate::alpha_normalize::{normalize_record, SourceRecord};
use std::collections::BTreeMap;

fn record(fields: &[(&str, &str)]) -> SourceRecord {
    SourceRecord {
        source_key: "test".to_owned(),
        sheet: "sheet".to_owned(),
        excel_row: 1,
        fields: fields
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>(),
    }
}

#[test]
fn rejected_urls_are_exceptions_without_raw_values() {
    let athlete = normalize_record(&record(&[
        ("athlete_id", "123"),
        ("profile_url", "https://example.com/athlete/123"),
        ("result_urls", "https://athletic.net/result/nope"),
        ("source_url", "https://example.com/source"),
    ]));
    assert!(athlete.profile_urls.is_empty());
    assert!(athlete.results.is_empty());
    assert!(athlete.source_urls.is_empty());
    assert!(athlete.exception_notes.len() >= 3);
    assert!(athlete.exception_notes.iter().all(|note| !note.contains("example.com")));
}

#[test]
fn result_ids_follow_marks_not_url_placeholders() {
    let athlete = normalize_record(&record(&[
        ("athlete_id", "123"),
        ("result_urls", "https://athletic.net/result/100"),
        ("marks", "100m|10.55|2026-27|2026-05-01|Meet"),
        ("result_ids", "999"),
    ]));
    assert_eq!(athlete.results.len(), 2);
    assert_eq!(athlete.results[0].result_id, Some(100));
    assert_eq!(athlete.results[1].result_id, Some(999));
}

#[test]
fn location_state_is_retained_and_contact_is_rejected() {
    let athlete = normalize_record(&record(&[
        ("athlete_id", "123"),
        ("city", "Los Angeles, CA"),
    ]));
    assert_eq!(athlete.city, "Los Angeles");
    assert_eq!(athlete.state, "CA");
    let private = normalize_record(&record(&[("athlete_id", "123"), ("city", "555-123-4567")]));
    assert!(private.city.is_empty());
    assert!(private.exception_notes.iter().any(|note| note.contains("invalid city")));
}

#[test]
fn malformed_dates_and_seasons_are_exceptions() {
    let athlete = normalize_record(&record(&[
        ("athlete_id", "123"),
        ("marks", "100m|10.55|2026/27|not-a-date|Meet"),
    ]));
    assert!(athlete.results.is_empty());
    assert!(athlete.exception_notes.iter().any(|note| note.contains("invalid mark")));
}

#[test]
fn mismatched_profile_is_not_retained() {
    let athlete = normalize_record(&record(&[
        ("athlete_id", "123"),
        ("profile_url", "https://athletic.net/athlete/999"),
    ]));
    assert!(athlete.profile_urls.is_empty());
    assert!(athlete.exception_notes.iter().any(|note| note.contains("conflicts")));
}
