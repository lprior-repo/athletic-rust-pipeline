use super::*;
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FIXTURE: &str = include_str!("../../tests/fixtures/ks/kshsaa_directory_a.json");

#[test]
fn parses_fixture_records() -> TestResult {
    let records = parse_records(FIXTURE)?;
    check!(eq; records.len(), 5, "fixture contains 5 records");
    Ok(())
}

#[test]
fn parses_school_from_record() -> TestResult {
    let records = parse_records(FIXTURE)?;
    let record = &records[0];

    let (school, id) = parse_school(record, "https://example.com/api", "2026-09-20")
        .ok_or("Abilene has a name")?;

    check!(eq; school.name, "Abilene HS");
    check!(eq; school.state, Some(UsJurisdiction::Kansas));
    check!(eq; school.association.as_deref(), Some("kshsaa"));
    check!(eq; school.classification.as_deref(), Some("4A"));
    check!(eq; school.enrollment, Some(467));
    check!(eq;
        school.school_website.as_deref(),
        Some("www.abileneschools.org")
    );
    check!(eq; school.city.as_deref(), Some("Abilene"));

    check!(eq; school.source_identities.len(), 1);
    check!(eq;
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "kshsaa".into()
        }
    );
    check!(eq; school.source_identities[0].id, "KSS0001");

    check!(eq; school.evidence.len(), 1);
    check!(eq;
        school.evidence[0].source.url.as_deref(),
        Some("https://example.com/api")
    );

    let normalized = normalize_name("Abilene HS");
    let expected_id = CanonicalSchool::mint(UsJurisdiction::Kansas, "Abilene HS", &normalized);
    check!(eq; id, expected_id);
    Ok(())
}

#[test]
fn parses_ad_coach_from_record() -> TestResult {
    let records = parse_records(FIXTURE)?;
    let record = &records[0];
    let (_, school_id) = parse_school(record, "https://example.com/api", "2026-09-20")
        .ok_or("Abilene has a name")?;

    let coach = parse_ad_coach(record, &school_id, "https://example.com/api", "2026-09-20")
        .ok_or("Abilene has an AD")?;

    check!(eq; coach.name, "Derek Berns");
    check!(eq; coach.role, CoachRole::AthleticDirector);
    check!(eq; coach.sport, None);
    check!(eq; coach.gender, Gender::Mixed);
    check!(eq;
        coach.professional_email.as_deref(),
        Some("dberns@abileneschools.org")
    );
    Ok(())
}

#[test]
fn honorific_stripped_from_ad_name() -> TestResult {
    let mut record = parse_records(FIXTURE)?[0].clone();
    record.ad_name = Some("Coach Derek Berns".to_string());
    let (_, school_id) =
        parse_school(&record, "https://example.com/api", "2026-09-20").ok_or("Abilene school")?;
    let coach = parse_ad_coach(&record, &school_id, "https://example.com/api", "2026-09-20")
        .ok_or("Abilene AD")?;
    check!(eq; coach.name, "Derek Berns");
    Ok(())
}

#[test]
fn school_with_no_class_or_enrollment_parses() -> TestResult {
    let records = parse_records(FIXTURE)?;
    let ms = &records[3];
    check!(eq; ms.school_name, "Abilene MS");
    check!(ms.class.is_none());
    check!(ms.enrollment.is_none());

    let (school, _) =
        parse_school(ms, "https://example.com/api", "2026-09-20").ok_or("Abilene MS school")?;
    check!(eq; school.name, "Abilene MS");
    check!(eq; school.classification, None);
    check!(eq; school.enrollment, None);
    Ok(())
}

#[test]
fn school_with_no_website_parses() -> TestResult {
    let records = parse_records(FIXTURE)?;
    let gb = &records[4];
    check!(eq; gb.school_name, "Great Bend HS");

    let (school, _) =
        parse_school(gb, "https://example.com/api", "2026-09-20").ok_or("Great Bend school")?;
    check!(eq; school.name, "Great Bend HS");
    check!(eq; school.school_website, None);
    Ok(())
}

#[test]
fn no_cell_phones_or_principal_data_in_entities() -> TestResult {
    let records = parse_records(FIXTURE)?;

    for record in &records {
        let (_, school_id) = parse_school(record, "https://example.com/api", "2026-09-20")
            .ok_or("fixture school")?;
        let coach = parse_ad_coach(record, &school_id, "https://example.com/api", "2026-09-20");

        let coach = coach.ok_or("fixture AD")?;
        check!(
            coach.phone.is_none(),
            "coach phone must be None for all records"
        );

        if let Some(email) = &coach.professional_email {
            let ad_cell = record.ad_cell.as_deref().map(|s| s.trim());
            let principal_name = record.principal_name.as_deref().map(|s| s.trim());
            check!(
                ad_cell != Some(email.as_str()),
                "coach email should not match AD cell number"
            );
            check!(
                principal_name != Some(email.as_str()),
                "coach email should not match principal name"
            );
        }
    }
    Ok(())
}

#[test]
fn empty_ad_name_yields_none() -> TestResult {
    let records = parse_records(FIXTURE)?;
    let mut record = records[0].clone();
    record.ad_name = Some("".to_string());
    let (_, school_id) =
        parse_school(&record, "https://example.com/api", "2026-09-20").ok_or("Abilene school")?;
    check!(parse_ad_coach(&record, &school_id, "https://example.com/api", "2026-09-20").is_none());
    Ok(())
}

#[test]
fn malformed_json_errors_out() {
    let result = parse_records("not json");
    assert!(result.is_err(), "malformed JSON must error, not panic");
}

#[test]
fn empty_json_array_returns_zero_rows() -> TestResult {
    let records = parse_records("[]")?;
    check!(eq; records.len(), 0);
    Ok(())
}

#[test]
fn fixture_ad_email_fill_rate() -> TestResult {
    let records = parse_records(FIXTURE)?;
    let with_email = records
        .iter()
        .filter(|r| r.ad_email.as_deref().is_some_and(|s| !s.trim().is_empty()))
        .count();
    check!(eq;
        with_email,
        records.len(),
        "all {} fixture records have AD email (100% fill rate)",
        records.len()
    );
    Ok(())
}
