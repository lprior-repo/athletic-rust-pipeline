/// Focused tests for normalization, URL validation, and deduplication.
use crate::alpha_normalize::{
    canonical_state, dedup_athletes, merge_athlete, normalize_record, normalize_whitespace,
    SourceRecord,
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
fn normalize_whitespace_collapses() {
    assert_eq!(normalize_whitespace("  hello    world  "), "hello world");
    assert_eq!(normalize_whitespace("no-spaces"), "no-spaces");
    assert_eq!(normalize_whitespace("   "), "");
}

#[test]
fn normalize_record_extract_fields() {
    let mut fields = BTreeMap::new();
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
    assert_eq!(athlete.first_name, "John Doe");
    assert_eq!(athlete.last_name, "Doe");
    assert_eq!(athlete.school, "Lincoln High");
    assert_eq!(athlete.state, "CA");
    assert_eq!(athlete.profile_url, "https://athletic.net/athlete/12345");
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

    // Record 1: 100m mark
    let mut fields1 = BTreeMap::new();
    fields1.insert("athlete_id".to_owned(), "12345".to_owned());
    fields1.insert("state".to_owned(), "ca".to_owned());
    fields1.insert(
        "marks".to_owned(),
        "100m|10.55|2026-27|2026-05-01|Invitational|+1.2".to_owned(),
    );
    fields1.insert(
        "result_urls".to_owned(),
        "https://athletic.net/result/100".to_owned(),
    );

    let rec1 = SourceRecord {
        source_key: "src1".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields: fields1,
    };

    // Record 2: 200m mark
    let mut fields2 = BTreeMap::new();
    fields2.insert("athlete_id".to_owned(), "12345".to_owned());
    fields2.insert("state".to_owned(), "ca".to_owned());
    fields2.insert(
        "marks".to_owned(),
        "200m|21.30|2026-27|2026-05-15|State Finals|+0.8".to_owned(),
    );
    fields2.insert(
        "result_urls".to_owned(),
        "https://athletic.net/result/200".to_owned(),
    );

    let rec2 = SourceRecord {
        source_key: "src2".to_owned(),
        sheet: "s2".to_owned(),
        excel_row: 2,
        fields: fields2,
    };

    let a1 = normalize_record(&rec1);
    let a2 = normalize_record(&rec2);
    merge_athlete(&mut map, a1);
    let id = merge_athlete(&mut map, a2);

    assert_eq!(id, 12345);
    let athlete = &map[&12345];
    assert_eq!(athlete.results.len(), 3);
    assert_eq!(
        athlete
            .results
            .iter()
            .filter(|r| !r.result_url.is_empty())
            .count(),
        2
    );
}

#[test]
fn duplicate_result_collapse_by_result_id() {
    let mut map = BTreeMap::new();
    let mut fields1 = BTreeMap::new();
    fields1.insert("athlete_id".to_owned(), "12345".to_owned());
    fields1.insert("result_ids".to_owned(), "999".to_owned());
    fields1.insert(
        "marks".to_owned(),
        "100m|10.55|2026-27|2026-05-01|Invitational".to_owned(),
    );

    let rec1 = SourceRecord {
        source_key: "src1".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields: fields1,
    };

    let mut fields2 = BTreeMap::new();
    fields2.insert("athlete_id".to_owned(), "12345".to_owned());
    fields2.insert("result_ids".to_owned(), "999".to_owned());
    fields2.insert(
        "marks".to_owned(),
        "100m|10.55|2026-27|2026-05-01|Invitational".to_owned(),
    );

    let rec2 = SourceRecord {
        source_key: "src2".to_owned(),
        sheet: "s2".to_owned(),
        excel_row: 2,
        fields: fields2,
    };

    let a1 = normalize_record(&rec1);
    let a2 = normalize_record(&rec2);
    merge_athlete(&mut map, a1);
    let _ = merge_athlete(&mut map, a2);

    let athlete = &map[&12345];
    assert_eq!(
        athlete
            .results
            .iter()
            .filter(|r| r.result_id == 999)
            .count(),
        1
    );
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

    let a1 = normalize_record(&rec1);
    let a2 = normalize_record(&rec2);
    merge_athlete(&mut map, a1);
    merge_athlete(&mut map, a2);

    let athlete = &map[&12345];
    assert!(!athlete.exception_notes.is_empty());
    assert!(athlete
        .exception_notes
        .iter()
        .any(|n| n.contains("first_name conflict")));
}

#[test]
fn missing_zero_id_creates_exception_record() {
    let mut fields = BTreeMap::new();
    fields.insert("first_name".to_owned(), "Jane Doe".to_owned());
    fields.insert("school".to_owned(), "West High".to_owned());
    fields.insert("state".to_owned(), "tx".to_owned());
    let record = SourceRecord {
        source_key: "bad".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };

    let mut map = BTreeMap::new();
    let a = normalize_record(&record);
    merge_athlete(&mut map, a);

    // Exception records go under key 0
    let exn = &map[&0];
    assert!(exn
        .exception_notes
        .iter()
        .any(|n| n.contains("athlete_id missing or zero")));
}

#[test]
fn state_unknown_rejected() {
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
fn profile_url_not_in_result_urls() {
    let mut fields = BTreeMap::new();
    fields.insert("athlete_id".to_owned(), "12345".to_owned());
    fields.insert(
        "profile_url".to_owned(),
        "https://athletic.net/athlete/12345".to_owned(),
    );
    fields.insert(
        "result_urls".to_owned(),
        "https://athletic.net/athlete/12345;https://athletic.net/result/100".to_owned(),
    );
    let record = SourceRecord {
        source_key: "test".to_owned(),
        sheet: "s1".to_owned(),
        excel_row: 1,
        fields,
    };
    let athlete = normalize_record(&record);
    assert!(!athlete
        .results
        .iter()
        .any(|r| r.result_url == athlete.profile_url));
    assert!(athlete
        .results
        .iter()
        .any(|r| r.result_url == "https://athletic.net/result/100"));
}

#[test]
fn dedup_athletes_keeps_first() {
    let mut fields1 = BTreeMap::new();
    fields1.insert("athlete_id".to_owned(), "12345".to_owned());
    fields1.insert("first_name".to_owned(), "John".to_owned());
    let a1 = normalize_record(&SourceRecord {
        source_key: "s1".into(),
        sheet: "s1".into(),
        excel_row: 1,
        fields: fields1,
    });

    let mut fields2 = BTreeMap::new();
    fields2.insert("athlete_id".to_owned(), "12345".to_owned());
    fields2.insert("first_name".to_owned(), "Jane".to_owned());
    let a2 = normalize_record(&SourceRecord {
        source_key: "s2".into(),
        sheet: "s2".into(),
        excel_row: 2,
        fields: fields2,
    });

    let deduped = dedup_athletes(vec![a1, a2]);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].first_name, "John");
}
