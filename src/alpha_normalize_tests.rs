/// Focused tests for normalization, URL validation, and deduplication.
use crate::alpha_normalize::{
    canonical_state, construct_profile_url, exception_for_missing_id, merge_athlete,
    normalize_record, normalize_whitespace, validate_url, SourceRecord,
};
use crate::model::Mark;
use std::collections::BTreeMap;

#[test]
fn canonical_state_uppercases() {
    assert_eq!(canonical_state("ca"), "CA");
    assert_eq!(canonical_state("  ca  "), "CA");
    assert_eq!(canonical_state("ny"), "NY");
}

#[test]
fn normalize_whitespace_collapses() {
    assert_eq!(normalize_whitespace("  hello    world  "), "hello world");
    assert_eq!(normalize_whitespace("no-spaces"), "no-spaces");
    assert_eq!(normalize_whitespace("   "), "");
}

#[test]
fn validate_url_athletic_net_https_ok() {
    assert!(validate_url("https://athletic.net/athlete/12345").is_some());
    assert!(validate_url("https://athletic.net/results/123").is_some());
}

#[test]
fn validate_url_rejects_non_https_and_unauthorized() {
    assert!(validate_url("http://athletic.net/athlete/123").is_none());
    assert!(validate_url("ftp://athletic.net/athlete/123").is_none());
    assert!(validate_url("https://other-site.com/athlete/123").is_none());
    assert!(validate_url("https://athletic.net.evil.com/athlete/123").is_none());
    assert!(validate_url("").is_none());
    assert!(validate_url("  ").is_none());
}

#[test]
fn construct_profile_url_nonzero_id() {
    assert_eq!(
        construct_profile_url(12345),
        Some("https://athletic.net/athlete/12345".to_owned())
    );
    assert!(construct_profile_url(0).is_none());
}

#[test]
fn normalize_record_extract_fields() {
    let mut fields = BTreeMap::new();
    fields.insert("first_name".to_owned(), "  John  ".to_owned());
    fields.insert("last_name".to_owned(), "Doe".to_owned());
    fields.insert("school".to_owned(), "  Lincoln High  ".to_owned());
    fields.insert("state".to_owned(), "ca".to_owned());
    fields.insert("location".to_owned(), "  LA  ".to_owned());
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
    assert_eq!(athlete.first_name, "John Doe");
    assert_eq!(athlete.school, "Lincoln High");
    assert_eq!(athlete.state, "CA");
    assert_eq!(athlete.location, "LA");
    assert_eq!(
        athlete.profile_url,
        "https://athletic.net/athlete/12345"
    );
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
        sheet: "sheet1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert_eq!(athlete.profile_url, "");
}

#[test]
fn merge_athlete_two_events_one_athlete_two_results() {
    let mut map = BTreeMap::new();

    let mut fields1 = BTreeMap::new();
    fields1.insert("athlete_id".to_owned(), "12345".to_owned());
    fields1.insert("state".to_owned(), "ca".to_owned());
    fields1.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/12345".to_owned(),
    );
    fields1.insert(
        "result_urls".to_owned(),
        "https://athletic.net/results/100".to_owned(),
    );

    let mut mark1 = Mark::default();
    mark1.event = "100m".to_owned();
    mark1.mark = "10.55".to_owned();
    mark1.date = "2026-05-01".to_owned();
    mark1.meet_name = "Invitational".to_owned();
    mark1.source_url = "https://athletic.net/results/100".to_owned();

    let rec1 = SourceRecord {
        source_key: "src1".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields: fields1,
    };

    let mut fields2 = BTreeMap::new();
    fields2.insert("athlete_id".to_owned(), "12345".to_owned());
    fields2.insert("state".to_owned(), "ca".to_owned());
    fields2.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/12345".to_owned(),
    );
    fields2.insert(
        "result_urls".to_owned(),
        "https://athletic.net/results/200".to_owned(),
    );

    let mut mark2 = Mark::default();
    mark2.event = "200m".to_owned();
    mark2.mark = "21.30".to_owned();
    mark2.date = "2026-05-15".to_owned();
    mark2.meet_name = "State Finals".to_owned();
    mark2.source_url = "https://athletic.net/results/200".to_owned();

    let rec2 = SourceRecord {
        source_key: "src2".to_owned(),
        sheet: "s2".to_owned(),
        excel_row: 2,
        fields: fields2,
    };

    let athlete1 = normalize_record(&rec1);
    let mut athlete1 = athlete1;
    athlete1.marks = vec![mark1];

    let athlete2 = normalize_record(&rec2);
    let mut athlete2 = athlete2;
    athlete2.marks = vec![mark2];

    merge_athlete(&mut map, athlete1);
    let id = merge_athlete(&mut map, athlete2);

    assert_eq!(id, 12345);
    let athlete = &map[&12345];
    assert_eq!(athlete.marks.len(), 2);
    assert_eq!(athlete.result_urls.len(), 2);
}

#[test]
fn duplicate_result_collapse() {
    let mut map = BTreeMap::new();
    let mut fields1 = BTreeMap::new();
    fields1.insert("athlete_id".to_owned(), "12345".to_owned());
    let rec1 = SourceRecord {
        source_key: "src1".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields: fields1,
    };

    let mut mark = Mark::default();
    mark.event = "100m".to_owned();
    mark.mark = "10.55".to_owned();
    mark.date = "2026-05-01".to_owned();
    mark.meet_name = "Invitational".to_owned();
    mark.source_url = "https://athletic.net/results/100".to_owned();

    let mut athlete1 = normalize_record(&rec1);
    athlete1.marks = vec![mark.clone()];

    let mut fields2 = BTreeMap::new();
    fields2.insert("athlete_id".to_owned(), "12345".to_owned());
    let rec2 = SourceRecord {
        source_key: "src2".to_owned(),
        sheet: "s2".to_owned(),
        excel_row: 2,
        fields: fields2,
    };

    let mut athlete2 = normalize_record(&rec2);
    athlete2.marks = vec![mark];

    merge_athlete(&mut map, athlete1);
    let _ = merge_athlete(&mut map, athlete2);

    let athlete = &map[&12345];
    assert_eq!(athlete.marks.len(), 1);
}

#[test]
fn identity_conflict_creates_exception() {
    let mut map = BTreeMap::new();
    let mut fields1 = BTreeMap::new();
    fields1.insert("athlete_id".to_owned(), "12345".to_owned());
    fields1.insert("first_name".to_owned(), "John".to_owned());
    let rec1 = SourceRecord {
        source_key: "src1".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields: fields1,
    };

    let mut fields2 = BTreeMap::new();
    fields2.insert("athlete_id".to_owned(), "12345".to_owned());
    fields2.insert("first_name".to_owned(), "Johnny".to_owned());
    let rec2 = SourceRecord {
        source_key: "src2".to_owned(),
        sheet: "s2".to_owned(),
        excel_row: 2,
        fields: fields2,
    };

    let athlete1 = normalize_record(&rec1);
    let athlete2 = normalize_record(&rec2);

    merge_athlete(&mut map, athlete1);
    merge_athlete(&mut map, athlete2);

    let athlete = &map[&12345];
    assert!(!athlete.exception_notes.is_empty());
    assert!(athlete.exception_notes.iter().any(|n| n.contains("first_name conflict")));
}

#[test]
fn missing_id_creates_exception_record() {
    let mut fields = BTreeMap::new();
    fields.insert("first_name".to_owned(), "Jane".to_owned());
    fields.insert("last_name".to_owned(), "Doe".to_owned());
    fields.insert("school".to_owned(), "West High".to_owned());
    fields.insert("state".to_owned(), "tx".to_owned());
    let record = SourceRecord {
        source_key: "bad".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };

    let athlete = exception_for_missing_id(&record);
    assert_eq!(athlete.athlete_id, 0);
    assert_eq!(athlete.first_name, "Jane Doe");
    assert_eq!(athlete.school, "West High");
    assert_eq!(athlete.state, "TX");
    assert!(athlete.exception_notes[0].contains("athlete_id missing or zero"));
}

#[test]
fn zero_id_creates_exception_record() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "0".to_owned());
    fields.insert("first_name".to_owned(), "Bob".to_owned());
    let record = SourceRecord {
        source_key: "bad".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };

    let athlete = exception_for_missing_id(&record);
    assert_eq!(athlete.athlete_id, 0);
    assert!(athlete.exception_notes[0].contains("athlete_id missing or zero"));
}
