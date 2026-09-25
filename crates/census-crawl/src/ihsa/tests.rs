use super::*;

/// Fixture: trimmed subset of the real `GET /v1/schools` response.
///
/// **Provenance** — URL:
/// `https://api.ihsa.org/v1/schools`
/// (the single request returns all 828 member schools; this is a 3-school trimmed subset.)
/// **Capture file**:
/// `tools/a29-coach/il-sample.json` (the school list portion).
const FIXTURE_SCHOOLS: &str = include_str!("../../tests/fixtures/ihsa/v1_schools.json");

/// Fixture: real `GET /v1/schools/0101/staff2` response (Abingdon-Avon HS).
///
/// **Provenance** — URL:
/// `https://api.ihsa.org/v1/schools/0101/staff2`
/// **Capture file**:
/// `tools/a29-coach/il-sample.json` (Abingdon-Avon entry, SchoolID 0101).
const FIXTURE_STAFF_RICH: &str = include_str!("../../tests/fixtures/ihsa/staff2_coach_rich.json");

/// Fixture: real `GET /v1/schools/0430/staff2` response (Unity Christian HS).
///
/// **Provenance** — URL:
/// `https://api.ihsa.org/v1/schools/0430/staff2`
/// **Capture file**:
/// `tools/a29-coach/il-sample.json` (Unity Christian entry, SchoolID 0430).
const FIXTURE_STAFF_OFFICE: &str =
    include_str!("../../tests/fixtures/ihsa/staff2_office_only.json");

// ── Schools parsing tests ──────────────────────────────────────────────

#[test]
fn parses_schools_fixture() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    assert_eq!(records.len(), 3, "fixture contains 3 schools");
}

#[test]
fn parses_school_name_city_id() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let abingdon = &records[0];

    assert_eq!(abingdon.school_id, "0101");
    assert_eq!(abingdon.name_formal, "Abingdon-Avon High School");
    assert_eq!(abingdon.city, "Abingdon");
}

#[test]
fn parses_school_into_canonical() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let abingdon = &records[0];

    let (school, id) = parse_school(abingdon, "https://example.com/api", "2026-09-19")
        .expect("Abingdon has a name");

    assert_eq!(school.name, "Abingdon-Avon High School");
    assert_eq!(school.state, Some(UsJurisdiction::Illinois));
    assert_eq!(school.association.as_deref(), Some("ihsa"));
    assert_eq!(school.city.as_deref(), Some("Abingdon"));
    // `/v1/schools` rows carry no `URL` field (see this capture-backed fixture), so the school
    // website stays unset for IHSA.
    assert_eq!(school.school_website, None);

    // Source identity carries the IHSA SchoolID.
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "ihsa".into()
        }
    );
    assert_eq!(school.source_identities[0].id, "0101");

    // School id is deterministic.
    let expected_id = CanonicalSchool::mint(
        UsJurisdiction::Illinois,
        "Abingdon-Avon High School",
        &normalize_name("Abingdon-Avon High School"),
    );
    assert_eq!(id, expected_id);
}

#[test]
fn school_url_becomes_school_website() {
    // Constructed input, not a capture: the live `/v1/schools` rows carry no `URL` field, but the
    // adapter still maps one when a row has it.
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

    let (parsed, _) = parse_school(&row, "https://example.com/api", "2026-09-19").unwrap();
    assert_eq!(parsed.name, "Example High School");
    assert_eq!(parsed.city.as_deref(), Some("Example"));
    assert_eq!(
        parsed.school_website.as_deref(),
        Some("https://www.example.org")
    );
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

// ── Staff parsing tests ─────────────────────────────────────────────────

#[test]
fn parses_staff_fixture_rich() {
    let all = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
    // Live payload for school 0101: six categories, 31 (PersonID, title) rows. Reid Kelso is
    // both ADs and listed once as official representative, so role rows outnumber people.
    assert_eq!(
        all.len(),
        31,
        "coach-rich fixture flattens to 31 staff rows"
    );
    let people: std::collections::BTreeSet<i64> = all.iter().map(|p| p.person_id).collect();
    assert_eq!(people.len(), 20, "those rows belong to 20 distinct people");
}

#[test]
fn parses_staff_fixture_office_only() {
    let all = parse_staff(FIXTURE_STAFF_OFFICE).expect("fixture must parse");
    // Live payload for school 0138: administration only, no athletics staff categories.
    assert_eq!(all.len(), 6, "office-only fixture has 6 staff rows");
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

// ── Coach entity construction tests ─────────────────────────────────────

#[test]
fn coach_entity_from_rich_fixture() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let abingdon = &records[0];
    let (_, school_id) = parse_school(abingdon, "https://example.com/api", "2026-09-19")
        .expect("Abingdon has a name");

    let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
    let coaches: Vec<_> = staff
        .iter()
        .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
        .collect();

    // Kept: 17 sport head coaches + 2 AD rows + 2 "unknown" coach titles = 21 entities.
    // Dropped: principals, superintendent, medical staff, official representative.
    assert_eq!(
        coaches.len(),
        21,
        "rich fixture yields 21 coach/AD entities"
    );

    // Verify head coaches have correct sport/gender mapping.
    let boys_xc = coaches
        .iter()
        .find(|c| {
            c.name.contains("Mink")
                && c.sport == Some(Sport::CrossCountry)
                && c.gender == Gender::Boys
        })
        .expect("should have a Boys Cross Country coach named Mink");
    assert_eq!(boys_xc.role, CoachRole::HeadCoach);
    // The address is not in the staff payload; it comes from the reveal endpoint.
    assert_eq!(boys_xc.professional_email, None);

    let girls_tf = coaches
        .iter()
        .find(|c| {
            c.name.contains("Rakestraw")
                && c.sport == Some(Sport::OutdoorTrack)
                && c.gender == Gender::Girls
        })
        .expect("should have a Girls Track & Field coach named Rakestraw");
    assert_eq!(girls_tf.role, CoachRole::HeadCoach);
    assert_eq!(girls_tf.professional_email, None);

    // Verify ADs have sport = None, gender = Mixed.
    let ad = coaches
        .iter()
        .find(|c| c.role == CoachRole::AthleticDirector)
        .expect("should have at least one AD");
    assert_eq!(ad.sport, None);
    assert_eq!(ad.gender, Gender::Mixed);
    assert_eq!(ad.name, "Reid Kelso", "honorific stripped from the AD name");
}

#[test]
fn email_reveal_parses_only_real_addresses() {
    // Live body: /v1/schools/0101/staff/96256/email → {"email":"jrakestraw@atown276.net"}
    assert_eq!(
        parse_email(r#"{"email":"jrakestraw@atown276.net"}"#).as_deref(),
        Some("jrakestraw@atown276.net")
    );
    // Blank / missing / malformed reveals store nothing.
    assert_eq!(parse_email(r#"{"email":"   "}"#), None);
    assert_eq!(parse_email(r#"{"email":null}"#), None);
    assert_eq!(parse_email("{}"), None);
    assert_eq!(parse_email("<html>blocked</html>"), None);
}

#[test]
fn kept_staff_rows_advertise_an_email() {
    let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let (_, school_id) =
        parse_school(&records[0], "https://example.com/api", "2026-09-19").unwrap();

    let kept: Vec<_> = staff
        .iter()
        .filter(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19").is_some())
        .collect();
    assert!(
        kept.iter().all(|p| p.has_email == Some(true)),
        "every kept row is flagged HasEmail, so each reveal can return an address"
    );
}

#[test]
fn office_roles_excluded_from_coaches() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let unity = &records[2]; // Unity Christian
    let (_, school_id) =
        parse_school(unity, "https://example.com/api", "2026-09-19").expect("Unity has a name");

    let staff = parse_staff(FIXTURE_STAFF_OFFICE).expect("fixture must parse");
    let coaches: Vec<_> = staff
        .iter()
        .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
        .collect();

    // Unity's live payload: 6 administration rows of which only the two AD rows are coaches.
    assert_eq!(
        coaches.len(),
        2,
        "only 2 AD entities from the office-only fixture"
    );

    // Verify non-coaching roles were excluded and the ADs survive.
    assert!(coaches
        .iter()
        .all(|c| c.role == CoachRole::AthleticDirector));
    assert!(coaches.iter().all(|c| c.name.contains("Ringstrand")));
    assert!(coaches.iter().all(|c| c.sport.is_none()));
    assert!(
        !coaches.iter().any(|c| c.name.contains("Secretary")),
        "office titles must not survive as names"
    );
}

#[test]
fn honorifics_stripped_from_names() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let abingdon = &records[0];
    let (_, school_id) = parse_school(abingdon, "https://example.com/api", "2026-09-19").unwrap();

    let mut person = parse_staff(FIXTURE_STAFF_RICH)
        .unwrap()
        .into_iter()
        .find(|p| p.default_title == "Boys Track & Field Head Coach")
        .expect("rich fixture publishes a boys track head coach");
    person.name = "Coach Justin Rakestraw".to_string();

    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19")
        .expect("coach parse should work with honorific");
    assert_eq!(
        coach.name, "Justin Rakestraw",
        "honorific 'Coach' should be stripped"
    );

    // Also test other honorifics.
    person.name = "Mr. Justin Rakestraw".to_string();
    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19").unwrap();
    assert_eq!(
        coach.name, "Justin Rakestraw",
        "honorific 'Mr.' should be stripped"
    );

    person.name = "Dr. Justin Rakestraw".to_string();
    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19").unwrap();
    assert_eq!(
        coach.name, "Justin Rakestraw",
        "honorific 'Dr.' should be stripped"
    );

    person.name = "Miss Megan Hildreth".to_string();
    let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19").unwrap();
    assert_eq!(
        coach.name, "Megan Hildreth",
        "honorific 'Miss' should be stripped"
    );
}

#[test]
fn no_cell_phones_or_personal_data_in_entities() {
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let abingdon = &records[0];
    let (_, school_id) = parse_school(abingdon, "https://example.com/api", "2026-09-19").unwrap();

    let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
    for person in &staff {
        if let Some(coach) =
            parse_coach(person, &school_id, "https://example.com/api", "2026-09-19")
        {
            assert!(coach.phone.is_none(), "coach phone must be None");
        }
        // Whether or not the role is kept, the payload's phone/fax never reach an entity: the
        // only carrier is `StaffPerson`, which the entity conversion does not read for phones.
    }
}

// ── Error handling tests ────────────────────────────────────────────────

#[test]
fn malformed_json_errors_out() {
    let result = parse_schools("not json");
    assert!(result.is_err(), "malformed JSON must error, not panic");

    let result = parse_staff("not json");
    assert!(result.is_err(), "malformed JSON must error, not panic");
}

#[test]
fn empty_json_object_parses_empty_lists() {
    // An empty schools object should yield 0 records, not error.
    let result = parse_schools("{\"data\": []}");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);

    // An empty staff object should yield 0 persons, not error.
    let result = parse_staff("{}");
    assert!(
        result.is_err(),
        "a staff payload without `data` is not a valid envelope"
    );
    let empty = parse_staff(r#"{"data":{}}"#).expect("empty data object parses");
    assert!(empty.is_empty());
}

// ── Coach relevance test ────────────────────────────────────────────────

#[test]
fn email_reveal_covers_every_retained_row_that_advertises_one() {
    // The reveal is a paid request, so it stays bounded — by the payload's own `HasEmail` flag
    // rather than by the sport: the census keeps every address a source publishes, so a football
    // coach's is as publishable as a distance coach's.
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let (_, school_id) =
        parse_school(&records[0], "https://example.com/api", "2026-09-19").unwrap();
    let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");

    // What the collector asks for: a row it retains whose payload advertises an address.
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

    assert_eq!(
        requested, retained,
        "the flag is the only bound: no retained row is passed over for its sport"
    );
    assert!(
        requested.contains(&5494),
        "the football head coach's advertised address is requested too"
    );
}

#[test]
fn track_and_cross_country_roles_are_kept() {
    // The census exists for TF/XC: every one of those roles in the rich fixture must survive
    // with the right sport and gender, and ADs must survive without a sport.
    let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
    let abingdon = &records[0];
    let (_, school_id) = parse_school(abingdon, "https://example.com/api", "2026-09-19").unwrap();

    let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
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
        assert_eq!(
            found, 1,
            "exactly one {gender:?} {sport:?} head coach is published"
        );
    }

    let ads = coaches
        .iter()
        .filter(|c| c.role == CoachRole::AthleticDirector)
        .count();
    assert_eq!(ads, 2, "boys and girls AD rows both survive");
}
