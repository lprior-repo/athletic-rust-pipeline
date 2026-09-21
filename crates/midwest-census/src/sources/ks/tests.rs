use super::*;

/// Trimmed fixture: 5 real KSHSAA records extracted from the full directory capture.
///
/// **Provenance** — URL:
/// `https://kshsaa-api.kshsaa.org/directory/search/name/a/`
/// (the `a` endpoint returns the full ~526-school directory; the capture below is a
/// trimmed subset of that single response.)
/// **Capture file**:
/// `/home/lewis/Downloads/midwest-tfxc-source-research/tools/a29-coach/ks-full.json`
/// (HTTP 200, captured via browser devtools / network monitor).
const FIXTURE: &str = include_str!("../../../tests/fixtures/ks/kshsaa_directory_a.json");

#[test]
fn parses_fixture_records() {
    let records = parse_records(FIXTURE).expect("fixture must parse");
    assert_eq!(records.len(), 5, "fixture contains 5 records");
}

#[test]
fn parses_school_from_record() {
    let records = parse_records(FIXTURE).expect("fixture must parse");
    let record = &records[0]; // Abilene HS

    let (school, id) =
        parse_school(record, "https://example.com/api", "2026-09-20").expect("Abilene has a name");

    assert_eq!(school.name, "Abilene HS");
    assert_eq!(school.state.as_deref(), Some("KS"));
    assert_eq!(school.association.as_deref(), Some("kshsaa"));
    assert_eq!(school.classification.as_deref(), Some("4A"));
    assert_eq!(school.enrollment, Some(467));
    assert_eq!(
        school.school_website.as_deref(),
        Some("www.abileneschools.org")
    );
    assert_eq!(school.city.as_deref(), Some("Abilene"));

    // Source identity carries the KSHSAA external id.
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "kshsaa".into()
        }
    );
    assert_eq!(school.source_identities[0].id, "KSS0001");

    // Evidence carries the source ref and observation date.
    assert_eq!(school.evidence.len(), 1);
    assert_eq!(
        school.evidence[0].source.url.as_deref(),
        Some("https://example.com/api")
    );

    // The school id is deterministic from state + normalized name.
    let normalized = normalize_name("Abilene HS");
    let expected_id = CanonicalSchool::mint("KS", "Abilene HS", &normalized);
    assert_eq!(id, expected_id);
}

#[test]
fn parses_ad_coach_from_record() {
    let records = parse_records(FIXTURE).expect("fixture must parse");
    let record = &records[0];
    let (_, school_id) =
        parse_school(record, "https://example.com/api", "2026-09-20").expect("Abilene has a name");

    let coach = parse_ad_coach(record, &school_id, "https://example.com/api", "2026-09-20")
        .expect("Abilene has an AD");

    assert_eq!(coach.name, "Derek Berns"); // honorific stripped if present
    assert_eq!(coach.role, CoachRole::AthleticDirector);
    assert_eq!(coach.sport, None);
    assert_eq!(coach.gender, Gender::Mixed);
    assert_eq!(
        coach.professional_email.as_deref(),
        Some("dberns@abileneschools.org")
    );
}

#[test]
fn honorific_stripped_from_ad_name() {
    let mut record = parse_records(FIXTURE).expect("fixture must parse")[0].clone();
    record.ad_name = Some("Coach Derek Berns".to_string());
    let (_, school_id) = parse_school(&record, "https://example.com/api", "2026-09-20").unwrap();
    let coach =
        parse_ad_coach(&record, &school_id, "https://example.com/api", "2026-09-20").unwrap();
    assert_eq!(coach.name, "Derek Berns");
}

#[test]
fn school_with_no_class_or_enrollment_parses() {
    // Abilene MS (index 3) has no Class or Enrollment.
    let records = parse_records(FIXTURE).expect("fixture must parse");
    let ms = &records[3];
    assert_eq!(ms.school_name, "Abilene MS");
    assert!(ms.class.is_none());
    assert!(ms.enrollment.is_none());

    let (school, _) = parse_school(ms, "https://example.com/api", "2026-09-20").unwrap();
    assert_eq!(school.name, "Abilene MS");
    assert_eq!(school.classification, None);
    assert_eq!(school.enrollment, None);
}

#[test]
fn school_with_no_website_parses() {
    // Great Bend HS (index 4) has no WebSite.
    let records = parse_records(FIXTURE).expect("fixture must parse");
    let gb = &records[4];
    assert_eq!(gb.school_name, "Great Bend HS");

    let (school, _) = parse_school(gb, "https://example.com/api", "2026-09-20").unwrap();
    assert_eq!(school.name, "Great Bend HS");
    assert_eq!(school.school_website, None);
}

#[test]
fn no_cell_phones_or_principal_data_in_entities() {
    // The fixture contains PrincipalName, PrincipalCell, ADCell, PresName, PresCell, etc.
    // The parsed entities must NOT contain any of those values.
    let records = parse_records(FIXTURE).expect("fixture must parse");

    for record in &records {
        let (_, school_id) = parse_school(record, "https://example.com/api", "2026-09-20").unwrap();
        let coach = parse_ad_coach(record, &school_id, "https://example.com/api", "2026-09-20");

        // Coach phone field must be None.
        let coach = coach.unwrap();
        assert!(
            coach.phone.is_none(),
            "coach phone must be None for all records"
        );

        // The professional_email should never match a cell number or principal name.
        if let Some(email) = &coach.professional_email {
            let ad_cell = record.ad_cell.as_deref().map(|s| s.trim());
            let principal_name = record.principal_name.as_deref().map(|s| s.trim());
            assert!(
                ad_cell != Some(email.as_str()),
                "coach email should not match AD cell number"
            );
            assert!(
                principal_name != Some(email.as_str()),
                "coach email should not match principal name"
            );
        }
    }
}

#[test]
fn empty_ad_name_yields_none() {
    let records = parse_records(FIXTURE).expect("fixture must parse");
    let mut record = records[0].clone();
    record.ad_name = Some("".to_string());
    let (_, school_id) = parse_school(&record, "https://example.com/api", "2026-09-20").unwrap();
    assert!(parse_ad_coach(&record, &school_id, "https://example.com/api", "2026-09-20").is_none());
}

#[test]
fn malformed_json_errors_out() {
    let result = parse_records("not json");
    assert!(result.is_err(), "malformed JSON must error, not panic");
}

#[test]
fn empty_json_array_returns_zero_rows() {
    let records = parse_records("[]").expect("empty array must parse");
    assert_eq!(records.len(), 0);
}

#[test]
fn fixture_ad_email_fill_rate() {
    // All 5 fixture records have an AD email — 100% fill rate.
    let records = parse_records(FIXTURE).expect("fixture must parse");
    let with_email = records
        .iter()
        .filter(|r| {
            r.ad_email
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        with_email,
        records.len(),
        "all {} fixture records have AD email (100% fill rate)",
        records.len()
    );
}
