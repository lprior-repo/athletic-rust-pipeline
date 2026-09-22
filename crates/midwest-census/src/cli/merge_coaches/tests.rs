use super::judge::resolves_sport;
use super::*;
use std::collections::BTreeSet;
use tempfile::NamedTempFile;

fn make_row(fields: Vec<&str>) -> Row {
    let s: Vec<String> = fields.iter().map(|s| s.to_string()).collect();
    Row::from_fields(s)
}

// ─── Normalization tests ─────────────────────────────────────────────

#[test]
fn multi_url_trims_to_first() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "",
        "",
        "",
        "https://first.example.com https://second.example.com",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.source_url, "https://first.example.com");
}

#[test]
fn personal_mail_blank() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@gmail.com",
        "",
        "ad@yahoo.com",
        "",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.public_professional_email, "");
    assert_eq!(row.ad_email, "");
}

#[test]
fn professional_mail_kept() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.k12.oh.us",
        "",
        "ad@school.k12.oh.us",
        "",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.public_professional_email, "coach@school.k12.oh.us");
    assert_eq!(row.ad_email, "ad@school.k12.oh.us");
}

#[test]
fn url_with_no_https_stays_untrimmed() {
    let mut row = make_row(vec![
        "School",
        "",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "",
        "",
        "",
        "not-a-url",
        "2026-09-22",
    ]);
    row.normalize();
    assert_eq!(row.source_url, "not-a-url");
}

// ─── Judge tests ─────────────────────────────────────────────────────

#[test]
fn director_with_empty_sport_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "",
        "Athletic Director",
        "",
        "",
        "",
        "ad@school.edu",
        "https://school.edu",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn director_with_nonempty_sport_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Track",
        "Athletic Director",
        "",
        "",
        "",
        "ad@school.edu",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("director row must leave sport empty"));
}

#[test]
fn coach_with_resolvable_sport_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Boys Cross Country",
        "Head Coach",
        "Coach Name",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn coach_with_indoor_sport_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Indoor Track",
        "Head Coach",
        "Coach Name",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn coach_with_unrelated_sport_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Football",
        "Head Coach",
        "Coach Name",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("no sport resolvable"));
}

#[test]
fn coach_with_no_name_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("no coach name"));
}

#[test]
fn role_neither_coach_nor_director_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Athletic Trainer",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("neither a coach nor a director"));
}

#[test]
fn role_both_coach_and_director_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "",
        "Assistant Athletic Director Coach",
        "Coach",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("both a coach and a director"));
}

#[test]
fn placeholder_names_rejected() {
    for placeholder in &[
        "vacant", "TBA", "TBD", "NA", "n/a", "none", "unknown", "---",
    ] {
        let row = make_row(vec![
            "School",
            "City",
            "OH",
            "Cross Country",
            "Head Coach",
            placeholder,
            "",
            "",
            "",
            "https://school.edu",
            "2026-09-22",
        ]);
        let reason = row.judge("OH").expect("expected rejection");
        assert!(
            reason.contains("placeholder name"),
            "expected placeholder rejection for {placeholder}, got: {reason}"
        );
    }
}

#[test]
fn phone_in_school_rejected() {
    let row = make_row(vec![
        "555-123-4567",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("phone-like value in school"));
}

#[test]
fn invalid_email_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "not-an-email",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("not an email"));
}

#[test]
fn personal_mail_domain_in_email_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@gmail.com",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("personal mail domain"));
}

#[test]
fn missing_school_rejected() {
    let row = make_row(vec![
        "",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("missing school"));
}

#[test]
fn no_contact_published_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "",
        "Athletic Director",
        "",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("no contact published"));
}

#[test]
fn state_mismatch_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "IN",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("does not match fragment"));
}

#[test]
fn valid_url_passes() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu/coaches",
        "2026-09-22",
    ]);
    assert!(row.judge("OH").is_none());
}

#[test]
fn invalid_url_rejected() {
    let row = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "not-a-url",
        "2026-09-22",
    ]);
    let reason = row.judge("OH").expect("expected rejection");
    assert!(reason.contains("not a URL"));
}

// ─── Resolves_sport tests ────────────────────────────────────────────

#[test]
fn resolves_track() {
    assert!(resolves_sport("Boys Track"));
    assert!(resolves_sport("Girls Track and Field"));
}

#[test]
fn resolves_cross_country() {
    assert!(resolves_sport("Cross Country"));
    assert!(resolves_sport("Boys Cross Country"));
    assert!(resolves_sport("Girls Cross-country"));
}

#[test]
fn resolves_indoor() {
    assert!(resolves_sport("Indoor Track"));
}

#[test]
fn does_not_resolve_soccer() {
    assert!(!resolves_sport("Soccer"));
    assert!(!resolves_sport("Basketball"));
}

// ─── Integration: file load + validate + output ──────────────────────

#[test]
fn merge_round_trip_keeps_every_row_importable_and_distinct() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("WI.csv"),
        "school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed\n\
         Madison West High School,Madison,WI,Track & Field,Head Coach,Dana Reed,,,,\
         https://madisonwest.example.org/athletics,2026-09-22\n\
         Madison West High School,Madison,WI,Cross Country,Head Coach,Sam Ellery,\
         sam.ellery@madisonwest.example.org,,,https://madisonwest.example.org/athletics,2026-09-22\n",
    )
    .expect("write WI fragment");
    std::fs::write(
        dir.path().join("MN.csv"),
        "school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed\n\
         Washburn High School,Minneapolis,MN,Track & Field,Head Coach,Rae Lindqvist,\
         rae.lindqvist@washburn.example.org,,,https://washburn.example.org/athletics,2026-09-22\n",
    )
    .expect("write MN fragment");
    // A second WI fragment repeats one row less richly: the merge keeps the richer copy once.
    std::fs::write(
        dir.path().join("WI-extra.csv"),
        "school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed\n\
         Madison West High School,Madison,WI,Track & Field,Head Coach,Dana Reed,\
         dana.reed@madisonwest.example.org,,,https://madisonwest.example.org/athletics,2026-09-22\n",
    )
    .expect("write second WI fragment");

    let out = NamedTempFile::new().expect("temp file");
    let report = NamedTempFile::new().expect("temp file");
    let args = MergeCoachesArgs {
        fragments: dir.path().to_path_buf(),
        out: out.path().to_path_buf(),
        report: report.path().to_path_buf(),
    };
    run_merge_coaches(&args).expect("merge_coaches");

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(out.path())
        .expect("from_path");
    let mut seen: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    let mut data_rows = 0usize;
    for record in reader.records() {
        let record = record.expect("record");
        let fields: Vec<String> = record.iter().map(str::to_owned).collect();
        let mut row = Row::from_fields(fields);
        row.normalize();
        data_rows += 1;
        assert!(
            super::judge::judge(&row, &row.state).is_none(),
            "merged row {data_rows} is not importable: {row:?}",
        );
        assert!(
            seen.insert(row.dedupe_key()),
            "merged row {data_rows} duplicates an earlier row after round-trip: {row:?}",
        );
    }
    assert_eq!(data_rows, 3, "two states, one deduped pair");
    let report = std::fs::read_to_string(report.path()).expect("read report");
    assert!(
        report.contains("WI") && report.contains("MN"),
        "the report accounts for both states: {report}"
    );
}

#[test]
fn dedupe_keeps_richer_row() {
    let mut row1 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    let mut row2 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach B",
        "coach@gmail.com",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    row1.normalize();
    row2.normalize();
    let key = row1.dedupe_key();
    assert_eq!(key, row2.dedupe_key());

    let mut kept: BTreeMap<_, _> = BTreeMap::new();
    kept.insert(key.clone(), row1.clone());
    let existing = kept.get(&key).expect("should exist");
    let new_has_email = !row2.public_professional_email.trim().is_empty();
    let old_has_email = !existing.public_professional_email.trim().is_empty();
    let new_is_newer = row2.last_observed.trim() > existing.last_observed.trim();
    if new_has_email && !old_has_email || new_is_newer {
        kept.insert(key.clone(), row2);
    }
    assert_eq!(kept[&key].coach_name, "Coach");
}

#[test]
fn dedupe_keeps_newer_row() {
    let mut row1 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach",
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-21",
    ]);
    let mut row2 = make_row(vec![
        "School",
        "City",
        "OH",
        "Cross Country",
        "Head Coach",
        "Coach B",
        "coach2@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    row1.normalize();
    row2.normalize();
    let key = row1.dedupe_key();
    assert_eq!(key, row2.dedupe_key());

    let mut kept: BTreeMap<_, _> = BTreeMap::new();
    kept.insert(key.clone(), row1.clone());
    let existing = kept.get(&key).expect("should exist");
    let new_is_newer = row2.last_observed.trim() > existing.last_observed.trim();
    if new_is_newer {
        kept.insert(key.clone(), row2);
    }
    assert_eq!(kept[&key].coach_name, "Coach B");
}
