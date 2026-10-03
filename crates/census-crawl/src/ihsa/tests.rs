use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FIXTURE_SCHOOLS: &str = include_str!("../../tests/fixtures/ihsa/v1_schools.json");

const FIXTURE_STAFF_RICH: &str = include_str!("../../tests/fixtures/ihsa/staff2_coach_rich.json");

const FIXTURE_STAFF_OFFICE: &str =
    include_str!("../../tests/fixtures/ihsa/staff2_office_only.json");

#[test]
fn parses_schools_fixture() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    check!(eq; records.len(), 3, "fixture contains 3 schools");
    Ok(())
}

#[test]
fn parses_school_name_city_id() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let abingdon = &records[0];

    check!(eq; abingdon.school_id, "0101");
    check!(eq; abingdon.name_formal, "Abingdon-Avon High School");
    check!(eq; abingdon.city, "Abingdon");
    Ok(())
}

#[test]
fn parses_school_into_canonical() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let abingdon = &records[0];

    let (school, id) = parse_school(abingdon, "https://example.com/api", "2026-09-19")
        .ok_or("Abingdon has a name")?;

    check!(eq; school.name, "Abingdon-Avon High School");
    check!(eq; school.state, Some(UsJurisdiction::Illinois));
    check!(eq; school.association.as_deref(), Some("ihsa"));
    check!(eq; school.city.as_deref(), Some("Abingdon"));
    check!(eq; school.school_website, None);

    check!(eq; school.source_identities.len(), 1);
    check!(eq;
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "ihsa".into()
        }
    );
    check!(eq; school.source_identities[0].id, "0101");

    let expected_id = CanonicalSchool::mint(
        UsJurisdiction::Illinois,
        "Abingdon-Avon High School",
        &normalize_name("Abingdon-Avon High School"),
    );
    check!(eq; id, expected_id);
    Ok(())
}

#[test]
fn school_url_becomes_school_website() -> TestResult {
    let row = SchoolRecord {
        school_id: "9999".into(),
        name_formal: "Example High School".into(),
        name_ihsa: None,
        name_short: None,
        city: "Example".into(),
        membership_type: None,
        r#type: None,
        enrollment_type: None,
        has_boundary: None,
        is_cps: None,
        url: Some("https://www.example.org".into()),
    };

    let (parsed, _) =
        parse_school(&row, "https://example.com/api", "2026-09-19").ok_or("Example school")?;
    check!(eq; parsed.name, "Example High School");
    check!(eq; parsed.city.as_deref(), Some("Example"));
    check!(eq;
        parsed.school_website.as_deref(),
        Some("https://www.example.org")
    );
    Ok(())
}

#[test]
fn school_with_blank_name_is_skipped() {
    let row = SchoolRecord {
        school_id: "9998".into(),
        name_formal: "   ".into(),
        name_ihsa: None,
        name_short: None,
        city: "Nowhere".into(),
        membership_type: None,
        r#type: None,
        enrollment_type: None,
        has_boundary: None,
        is_cps: None,
        url: None,
    };

    assert!(parse_school(&row, "https://example.com/api", "2026-09-19").is_none());
}

#[test]
fn parses_staff_fixture_rich() -> TestResult {
    let all = parse_staff(FIXTURE_STAFF_RICH)?;
    check!(eq;
        all.len(),
        31,
        "coach-rich fixture flattens to 31 staff rows"
    );
    let people: std::collections::BTreeSet<i64> = all.iter().map(|p| p.person_id).collect();
    check!(eq; people.len(), 20, "those rows belong to 20 distinct people");
    Ok(())
}

#[test]
fn parses_staff_fixture_office_only() -> TestResult {
    let all = parse_staff(FIXTURE_STAFF_OFFICE)?;
    check!(eq; all.len(), 6, "office-only fixture has 6 staff rows");
    Ok(())
}

#[test]
fn coach_title_maps_sport_and_gender() {
    assert_eq!(
        parse_coach_title("Boys Cross Country Head Coach"),
        Some((Sport::CrossCountry, Gender::Boys))
    );
    assert_eq!(
        parse_coach_title("Girls Cross Country Head Coach"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        parse_coach_title("Boys Track & Field Head Coach"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        parse_coach_title("Girls Track & Field Head Coach"),
        Some((Sport::OutdoorTrack, Gender::Girls))
    );
}

#[test]
fn coach_title_returns_none_for_non_coaching() {
    assert!(parse_coach_title("Boys Athletic Director").is_none());
    assert!(parse_coach_title("Girls Athletic Director").is_none());
    assert!(parse_coach_title("Boys Athletic Director's Assistant").is_none());
    assert!(parse_coach_title("Girls Athletic Director's Assistant").is_none());
    assert!(parse_coach_title("Athletic Trainer").is_none());
    assert!(parse_coach_title("Principal").is_none());
    assert!(parse_coach_title("Athletic Director Secretary").is_none());
}

#[test]
fn role_parsed_for_coaching_and_ad() {
    assert_eq!(
        parse_role("Boys Cross Country Head Coach"),
        Some(CoachRole::HeadCoach)
    );
    assert_eq!(
        parse_role("Girls Track & Field Head Coach"),
        Some(CoachRole::HeadCoach)
    );
    assert_eq!(
        parse_role("Boys Athletic Director"),
        Some(CoachRole::AthleticDirector)
    );
    assert_eq!(
        parse_role("Girls Athletic Director"),
        Some(CoachRole::AthleticDirector)
    );
}

#[test]
fn role_returns_none_for_office_roles() {
    assert!(parse_role("Boys Athletic Director's Assistant").is_none());
    assert!(parse_role("Girls Athletic Director's Assistant").is_none());
    assert!(parse_role("Athletic Trainer").is_none());
    assert!(parse_role("Principal").is_none());
    assert!(parse_role("Athletic Director Secretary").is_none());
}

#[test]
fn coach_entity_from_rich_fixture() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let abingdon = &records[0];
    let (_, school_id) = parse_school(abingdon, "https://example.com/api", "2026-09-19")
        .ok_or("Abingdon has a name")?;

    let staff = parse_staff(FIXTURE_STAFF_RICH)?;
    let coaches: Vec<_> = staff
        .iter()
        .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
        .collect();

    check!(eq;
        coaches.len(),
        21,
        "rich fixture yields 21 coach/AD entities"
    );

    let boys_xc = coaches
        .iter()
        .find(|c| {
            c.name.contains("Mink")
                && c.sport == Some(Sport::CrossCountry)
                && c.gender == Gender::Boys
        })
        .ok_or("should have a Boys Cross Country coach named Mink")?;
    check!(eq; boys_xc.role, CoachRole::HeadCoach);
    check!(eq; boys_xc.professional_email, None);

    let girls_tf = coaches
        .iter()
        .find(|c| {
            c.name.contains("Rakestraw")
                && c.sport == Some(Sport::OutdoorTrack)
                && c.gender == Gender::Girls
        })
        .ok_or("should have a Girls Track & Field coach named Rakestraw")?;
    check!(eq; girls_tf.role, CoachRole::HeadCoach);
    check!(eq; girls_tf.professional_email, None);

    let ad = coaches
        .iter()
        .find(|c| c.role == CoachRole::AthleticDirector)
        .ok_or("should have at least one AD")?;
    check!(eq; ad.sport, None);
    check!(eq; ad.gender, Gender::Mixed);
    check!(eq; ad.name, "Reid Kelso", "honorific stripped from the AD name");
    Ok(())
}

#[test]
fn email_reveal_parses_only_real_addresses() {
    assert_eq!(
        parse_email(r#"{"email":"jrakestraw@atown276.net"}"#).as_deref(),
        Some("jrakestraw@atown276.net")
    );
    assert_eq!(parse_email(r#"{"email":"   "}"#), None);
    assert_eq!(parse_email(r#"{"email":null}"#), None);
    assert_eq!(parse_email("{}"), None);
    assert_eq!(parse_email("<html>blocked</html>"), None);
}

#[test]
fn kept_staff_rows_advertise_an_email() -> TestResult {
    let staff = parse_staff(FIXTURE_STAFF_RICH)?;
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let (_, school_id) =
        parse_school(&records[0], "https://example.com/api", "2026-09-19").ok_or("first school")?;

    let kept: Vec<_> = staff
        .iter()
        .filter(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19").is_some())
        .collect();
    check!(
        kept.iter().all(|p| p.has_email == Some(true)),
        "every kept row is flagged HasEmail, so each reveal can return an address"
    );
    Ok(())
}

#[test]
fn office_roles_excluded_from_coaches() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let unity = &records[2];
    let (_, school_id) =
        parse_school(unity, "https://example.com/api", "2026-09-19").ok_or("Unity has a name")?;

    let staff = parse_staff(FIXTURE_STAFF_OFFICE)?;
    let coaches: Vec<_> = staff
        .iter()
        .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
        .collect();

    check!(eq;
        coaches.len(),
        2,
        "only 2 AD entities from the office-only fixture"
    );

    check!(coaches
        .iter()
        .all(|c| c.role == CoachRole::AthleticDirector));
    check!(coaches.iter().all(|c| c.name.contains("Ringstrand")));
    check!(coaches.iter().all(|c| c.sport.is_none()));
    check!(
        !coaches.iter().any(|c| c.name.contains("Secretary")),
        "office titles must not survive as names"
    );
    Ok(())
}

#[test]
fn honorifics_stripped_from_names() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let abingdon = &records[0];
    let (_, school_id) =
        parse_school(abingdon, "https://example.com/api", "2026-09-19").ok_or("Abingdon school")?;

    let mut person = parse_staff(FIXTURE_STAFF_RICH)?
        .into_iter()
        .find(|p| p.default_title == "Boys Track & Field Head Coach")
        .ok_or("rich fixture publishes a boys track head coach")?;
    person.name = "Coach Justin Rakestraw".to_string();

    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19")
        .ok_or("coach parse should work with honorific")?;
    check!(eq;
        coach.name, "Justin Rakestraw",
        "honorific 'Coach' should be stripped"
    );

    person.name = "Mr. Justin Rakestraw".to_string();
    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19")
        .ok_or("Mr. coach")?;
    check!(eq;
        coach.name, "Justin Rakestraw",
        "honorific 'Mr.' should be stripped"
    );

    person.name = "Dr. Justin Rakestraw".to_string();
    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19")
        .ok_or("Dr. coach")?;
    check!(eq;
        coach.name, "Justin Rakestraw",
        "honorific 'Dr.' should be stripped"
    );

    person.name = "Miss Megan Hildreth".to_string();
    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19")
        .ok_or("Miss coach")?;
    check!(eq;
        coach.name, "Megan Hildreth",
        "honorific 'Miss' should be stripped"
    );
    Ok(())
}

#[test]
fn no_cell_phones_or_personal_data_in_entities() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let abingdon = &records[0];
    let (_, school_id) =
        parse_school(abingdon, "https://example.com/api", "2026-09-19").ok_or("Abingdon school")?;

    let staff = parse_staff(FIXTURE_STAFF_RICH)?;
    for person in &staff {
        if let Some(coach) =
            parse_coach(person, &school_id, "https://example.com/api", "2026-09-19")
        {
            check!(coach.phone.is_none(), "coach phone must be None");
        }
    }
    Ok(())
}

#[test]
fn malformed_json_errors_out() {
    let result = parse_schools("not json");
    assert!(result.is_err(), "malformed JSON must error, not panic");

    let result = parse_staff("not json");
    assert!(result.is_err(), "malformed JSON must error, not panic");
}

#[test]
fn empty_json_object_parses_empty_lists() -> TestResult {
    let result = parse_schools("{\"data\": []}");
    check!(result.is_ok());
    check!(eq; result?.len(), 0);

    let result = parse_staff("{}");
    check!(
        result.is_err(),
        "a staff payload without `data` is not a valid envelope"
    );
    let empty = parse_staff(r#"{"data":{}}"#)?;
    check!(empty.is_empty());
    Ok(())
}

#[test]
fn email_reveal_covers_every_retained_row_that_advertises_one() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let (_, school_id) =
        parse_school(&records[0], "https://example.com/api", "2026-09-19").ok_or("first school")?;
    let staff = parse_staff(FIXTURE_STAFF_RICH)?;

    let requested: std::collections::BTreeSet<i64> = staff
        .iter()
        .filter(|person| {
            person.has_email == Some(true)
                && parse_coach(person, &school_id, "https://example.com/api", "2026-09-19")
                    .is_some()
        })
        .map(|person| person.person_id)
        .collect();
    let retained: std::collections::BTreeSet<i64> = staff
        .iter()
        .filter_map(|person| {
            parse_coach(person, &school_id, "https://example.com/api", "2026-09-19")
                .map(|_| person.person_id)
        })
        .collect();

    check!(eq;
        requested, retained,
        "the flag is the only bound: no retained row is passed over for its sport"
    );
    check!(
        requested.contains(&5494),
        "the football head coach's advertised address is requested too"
    );
    Ok(())
}

#[test]
fn track_and_cross_country_roles_are_kept() -> TestResult {
    let records = parse_schools(FIXTURE_SCHOOLS)?;
    let abingdon = &records[0];
    let (_, school_id) =
        parse_school(abingdon, "https://example.com/api", "2026-09-19").ok_or("Abingdon school")?;

    let staff = parse_staff(FIXTURE_STAFF_RICH)?;
    let coaches: Vec<_> = staff
        .iter()
        .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
        .collect();

    for (sport, gender) in [
        (Sport::CrossCountry, Gender::Boys),
        (Sport::CrossCountry, Gender::Girls),
        (Sport::OutdoorTrack, Gender::Boys),
        (Sport::OutdoorTrack, Gender::Girls),
    ] {
        let found = coaches
            .iter()
            .filter(|c| {
                c.sport == Some(sport) && c.gender == gender && c.role == CoachRole::HeadCoach
            })
            .count();
        check!(eq;
            found, 1,
            "exactly one {gender:?} {sport:?} head coach is published"
        );
    }

    let ads = coaches
        .iter()
        .filter(|c| c.role == CoachRole::AthleticDirector)
        .count();
    check!(eq; ads, 2, "boys and girls AD rows both survive");
    Ok(())
}
