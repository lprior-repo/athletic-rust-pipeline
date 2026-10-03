use super::judge::resolves_sport;
use super::*;
use std::collections::BTreeSet;
use tempfile::NamedTempFile;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn make_row(fields: Vec<&str>) -> Row {
    let s: Vec<String> = fields.iter().map(|s| s.to_string()).collect();
    Row::from_fields(s)
}

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
fn director_with_nonempty_sport_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("director row must leave sport empty"));
    Ok(())
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
fn coach_with_unrelated_sport_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("no sport resolvable"));
    Ok(())
}

#[test]
fn coach_with_no_name_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("no coach name"));
    Ok(())
}

#[test]
fn role_neither_coach_nor_director_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("neither a coach nor a director"));
    Ok(())
}

#[test]
fn role_both_coach_and_director_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("both a coach and a director"));
    Ok(())
}

#[test]
fn placeholder_names_rejected() -> TestResult {
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
        let reason = row.judge("OH").ok_or("expected rejection")?;
        check!(
            reason.contains("placeholder name"),
            "expected placeholder rejection for {placeholder}, got: {reason}"
        );
    }
    Ok(())
}

#[test]
fn phone_in_school_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("phone-like value in school"));
    Ok(())
}

#[test]
fn invalid_email_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("not an email"));
    Ok(())
}

#[test]
fn personal_mail_domain_in_email_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("personal mail domain"));
    Ok(())
}

#[test]
fn missing_school_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("missing school"));
    Ok(())
}

#[test]
fn no_contact_published_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("no contact published"));
    Ok(())
}

#[test]
fn state_mismatch_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("does not match fragment"));
    Ok(())
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
fn invalid_url_rejected() -> TestResult {
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
    let reason = row.judge("OH").ok_or("expected rejection")?;
    check!(reason.contains("not a URL"));
    Ok(())
}

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

#[test]
fn merge_round_trip_keeps_every_row_importable_and_distinct() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = "school,city,state,sport,role,coach_name,public_professional_email,ad_name,\
                  ad_email,source_url,last_observed,verified_proof_digest\n";
    let proof = "9f2c1d4b7a3e50618c9d2f4a6b8e0c1d3f5a7b9c1d3e5f70819a2b3c4d5e6f70";
    std::fs::write(
        dir.path().join("WI.csv"),
        format!(
            "{header}\
             Madison West High School,Madison,WI,Track & Field,Head Coach,Dana Reed,,,,\
             https://madisonwest.example.org/athletics,2026-09-22,{proof}\n\
             Madison West High School,Madison,WI,Track & Field,Head Coach,Dana Reed,\
             dana.reed@madisonwest.example.org,,,https://madisonwest.example.org/athletics,\
             2026-09-22,{proof}\n\
             Madison West High School,Madison,WI,Cross Country,Head Coach,Sam Ellery,\
             sam.ellery@madisonwest.example.org,,,https://madisonwest.example.org/athletics,\
             2026-09-22,{proof}\n"
        ),
    )?;
    std::fs::write(
        dir.path().join("MN.csv"),
        format!(
            "{header}\
             Washburn High School,Minneapolis,MN,Track & Field,Head Coach,Rae Lindqvist,\
             rae.lindqvist@washburn.example.org,,,https://washburn.example.org/athletics,\
             2026-09-22,{proof}\n"
        ),
    )?;

    let out = NamedTempFile::new()?;
    let report = NamedTempFile::new()?;
    let args = MergeCoachesArgs {
        fragments: dir.path().to_path_buf(),
        out: out.path().to_path_buf(),
        report: report.path().to_path_buf(),
    };
    run_merge_coaches(&args)?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(out.path())?;
    let mut seen: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    let mut data_rows = 0usize;
    for record in reader.records() {
        let record = record?;
        let fields: Vec<String> = record.iter().map(str::to_owned).collect();
        let mut row = Row::from_fields(fields);
        row.normalize();
        data_rows += 1;
        check!(
            super::judge::judge(&row, &row.state).is_none(),
            "merged row {data_rows} is not importable: {row:?}",
        );
        check!(
            seen.insert(row.dedupe_key()),
            "merged row {data_rows} duplicates an earlier row after round-trip: {row:?}",
        );
    }
    check!(eq; data_rows, 3, "two states, one deduped pair");
    let report = std::fs::read_to_string(report.path())?;
    check!(
        report.contains("WI") && report.contains("MN"),
        "the report accounts for both states: {report}"
    );
    Ok(())
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
        "coach@school.edu",
        "",
        "",
        "https://school.edu",
        "2026-09-22",
    ]);
    row1.normalize();
    row2.normalize();
    let key = row1.dedupe_key();
    assert_eq!(key, row2.dedupe_key());
    assert!(
        pick_richer(&row2, &row1),
        "the row that publishes a professional email is the richer one"
    );
    assert!(
        !pick_richer(&row1, &row2),
        "a row without an email never replaces the row that publishes one"
    );
    let kept = if pick_richer(&row2, &row1) {
        &row2
    } else {
        &row1
    };
    assert_eq!(kept.coach_name, "Coach B");
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
    assert!(
        pick_richer(&row2, &row1),
        "the later observation wins when both publish an email"
    );
    assert!(
        !pick_richer(&row1, &row2),
        "an older observation never replaces a newer one"
    );
    let kept = if pick_richer(&row2, &row1) {
        &row2
    } else {
        &row1
    };
    assert_eq!(kept.coach_name, "Coach B");
}
