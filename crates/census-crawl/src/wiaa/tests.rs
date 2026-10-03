use super::*;
use census_domain::UsJurisdiction;

use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const INDEX_A: &str = include_str!("../../tests/fixtures/wiaa/directory_letter_a.html");

const SCHOOL_ABBOTSFORD: &str =
    include_str!("../../tests/fixtures/wiaa/school_org1_abbotsford.html");

const SCHOOL_GET: &str =
    include_str!("../../tests/fixtures/wiaa/school_org135_gale_ettrick_trempealeau.html");

const SCHOOL_SAILS: &str =
    include_str!("../../tests/fixtures/wiaa/school_org5151_sails_charter.html");

const OBSERVED_ON: &str = "2026-09-20";

fn entry_for(org_id: &str) -> IndexEntry {
    match parse_directory_letter(INDEX_A)
        .into_iter()
        .find(|entry| entry.org_id == org_id)
    {
        Some(value) => value,
        None => IndexEntry {
            org_id: org_id.to_string(),
            ..IndexEntry::default()
        },
    }
}

fn extract_for(org_id: &str, fixture: &str) -> TestResult<SchoolExtract> {
    let entry = entry_for(org_id);
    let page = parse_school_page(fixture);
    school_entities(&entry, &page, OBSERVED_ON).ok_or_else(|| "fixture page yields entities".into())
}

#[test]
fn index_parsing_yields_org_ids_level_and_city() -> TestResult {
    let entries = parse_directory_letter(INDEX_A);
    check!(eq; entries.len(), 6, "fixture keeps six real index rows");
    check!(eq; entries[0].org_id, "1");
    check!(eq; entries[0].name, "ABBOTSFORD");
    check!(eq; entries[0].level, "High School");
    check!(eq; entries[0].city, "Abbotsford");
    check!(eq;
        entries[0].page_url(),
        format!("{HOST}{SCHOOL_PATH}?orgID=1")
    );
    let last = entries.last().ok_or("six entries")?;
    check!(eq; last.name, "ADVANCED LEARNING ACADEMY OF WISCONSIN CHARTER");
    check!(eq; last.level, "High School");
    check!(entries.iter().any(|entry| entry.level == "Middle School"));
    let mut ids: Vec<&str> = entries.iter().map(|entry| entry.org_id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    check!(eq; ids.len(), entries.len(), "orgIDs are unique");
    check!(entries.iter().all(|entry| !entry.org_id.is_empty()));
    Ok(())
}

#[test]
fn empty_index_fragment_yields_no_rows() {
    assert!(parse_directory_letter("").is_empty());
    assert!(parse_directory_letter("null").is_empty());
    assert!(parse_directory_letter("<div class=\"alert\"></div>").is_empty());
}

#[test]
fn school_page_yields_name_city_conference_ad_and_identity() -> TestResult {
    let page = parse_school_page(SCHOOL_ABBOTSFORD);
    check!(eq; page.name, "Abbotsford");
    check!(eq; page.city.as_deref(), Some("Abbotsford"));
    check!(eq; page.conference.as_deref(), Some("Marawood"));
    check!(eq; page.level.as_deref(), Some("High School"));
    check!(eq; page.enrollment, Some(214));
    check!(eq;
        page.website.as_deref(),
        Some("http://www.abbotsford.k12.wi.us")
    );

    let extract = extract_for("1", SCHOOL_ABBOTSFORD)?;
    check!(eq; extract.school.name, "Abbotsford");
    check!(eq; extract.school.state, Some(UsJurisdiction::Wisconsin));
    check!(eq; extract.school.city.as_deref(), Some("Abbotsford"));
    check!(eq; extract.school.association.as_deref(), Some("wiaa"));
    check!(eq; extract.school.classification.as_deref(), Some("Marawood"));
    check!(eq; extract.school.enrollment, Some(214));
    check!(eq;
        extract.school.school_website.as_deref(),
        Some("http://www.abbotsford.k12.wi.us")
    );
    check!(eq;
        extract.school.id,
        CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Abbotsford",
            normalize_name("Abbotsford")
        )
        .1
    );
    let identity = extract
        .school
        .source_identities
        .first()
        .ok_or("one provider identity")?;
    check!(eq;
        identity.namespace,
        SourceNamespace::AssociationSchool {
            association: "wiaa".to_string()
        }
    );
    check!(eq; identity.id, "1");
    check!(eq;
        identity.url.as_deref(),
        Some("https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1")
    );
    check!(eq; extract.school.evidence.len(), 1);
    check!(eq; extract.school.evidence[0].observed_on, OBSERVED_ON);
    check!(eq; extract.school.evidence[0].source.id, SOURCE_ID);
    check!(extract.school.evidence[0]
        .source
        .url
        .as_deref()
        .is_some_and(|url| url.contains("orgID=1")));

    check!(!page.admins.is_empty(), "Abbotsford publishes office rows");
    for admin in &page.admins {
        check!(
            !admin.role.contains('<') && !admin.role.contains('>'),
            "markup in admin role {:?}",
            admin.role
        );
    }
    for coach in &page.coaches {
        check!(
            !coach.sport.contains('<') && !coach.role.contains('<'),
            "markup in coach row {:?} {:?}",
            coach.sport,
            coach.role
        );
    }

    let ad = extract
        .coaches
        .iter()
        .find(|coach| coach.role == CoachRole::AthleticDirector)
        .ok_or("Abbotsford publishes an athletic director")?;
    check!(eq; ad.name, "Alex Larson");
    check!(eq; ad.sport, None, "an AD is school-wide, never sport-bound");
    check!(eq; ad.gender, Gender::Mixed);
    check!(eq;
        ad.professional_email.as_deref(),
        Some("alarson@abbotsford.k12.wi.us")
    );
    check!(eq; ad.evidence.len(), 1);
    check!(eq; ad.evidence[0].observed_on, OBSERVED_ON);
    Ok(())
}

#[test]
fn coach_rows_map_to_sport_and_gender() -> TestResult {
    let extract = extract_for("1", SCHOOL_ABBOTSFORD)?;
    let find = |name: &str| -> Vec<&CanonicalCoach> {
        extract
            .coaches
            .iter()
            .filter(|coach| coach.name == name)
            .collect()
    };

    let knapmiller = find("JACOB KNAPMILLER");
    check!(eq; knapmiller.len(), 2, "same person, two sport-gender rows");
    let mut pairs: Vec<(Sport, Gender)> = knapmiller
        .iter()
        .map(|coach| Ok((coach.sport.ok_or("sport-bound")?, coach.gender)))
        .collect::<TestResult<_>>()?;
    pairs.sort_unstable();
    check!(eq;
        pairs,
        vec![
            (Sport::OutdoorTrack, Gender::Boys),
            (Sport::OutdoorTrack, Gender::Girls)
        ]
    );
    check!(knapmiller
        .iter()
        .all(|coach| coach.role == CoachRole::HeadCoach));
    check!(knapmiller.iter().all(|coach| {
        coach.professional_email.as_deref() == Some("jknapmiller@abbotsford.k12.wi.us")
    }));

    let novak = find("Dillon Novak");
    check!(eq; novak.len(), 1);
    check!(eq; novak[0].sport, Some(Sport::CrossCountry));
    check!(eq; novak[0].gender, Gender::Girls);
    check!(eq; novak[0].role, CoachRole::HeadCoach);
    check!(eq;
        novak[0].professional_email.as_deref(),
        Some("dnovak@abbotsford.k12.wi.us")
    );

    check!(eq;
        extract.coaches.len(),
        4,
        "one AD + three TF/XC head coaches"
    );
    check!(eq; extract.skipped_coach_rows, 11);
    for coach in &extract.coaches {
        if coach.role == CoachRole::AthleticDirector {
            continue;
        }
        check!(matches!(
            coach.sport,
            Some(Sport::OutdoorTrack | Sport::CrossCountry)
        ));
    }

    let gets = extract_for("135", SCHOOL_GET)?;
    let gold: Vec<&CanonicalCoach> = gets
        .coaches
        .iter()
        .filter(|coach| coach.name == "Paula Gold")
        .collect();
    let mut gold_pairs: Vec<(String, Gender)> = gold
        .iter()
        .map(|coach| -> TestResult<_> {
            let sport = coach.sport.ok_or("sport-bound")?;
            Ok((format!("{sport:?}"), coach.gender))
        })
        .collect::<TestResult<_>>()?;
    gold_pairs.sort();
    check!(eq;
        gold_pairs,
        vec![
            ("CrossCountry".to_string(), Gender::Boys),
            ("CrossCountry".to_string(), Gender::Girls),
            ("OutdoorTrack".to_string(), Gender::Girls),
        ]
    );
    check!(eq; gets.coaches.len(), 5, "one AD + four TF/XC head coaches");
    check!(eq; gets.skipped_coach_rows, 19);
    Ok(())
}

#[test]
fn sport_and_role_labels_are_mapped_strictly() {
    assert_eq!(
        parse_sport_label("Boys Track and Field"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        parse_sport_label("Girls Track and Field"),
        Some((Sport::OutdoorTrack, Gender::Girls))
    );
    assert_eq!(
        parse_sport_label("Boys Cross Country"),
        Some((Sport::CrossCountry, Gender::Boys))
    );
    assert_eq!(
        parse_sport_label("Girls Cross Country"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        parse_sport_label("Coed Track and Field"),
        Some((Sport::OutdoorTrack, Gender::Mixed))
    );
    assert_eq!(parse_sport_label("Boys Wrestling"), None);
    assert_eq!(parse_sport_label("Girls Swimming & Diving"), None);
    assert_eq!(parse_sport_label(""), None);

    assert_eq!(parse_coach_role("Head Coach"), Some(CoachRole::HeadCoach));
    assert_eq!(
        parse_coach_role("Assistant Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(parse_coach_role("Volunteer"), None);
    assert_eq!(parse_coach_role(""), None);

    assert_eq!(
        parse_admin_role("Athletic Director"),
        Some(CoachRole::AthleticDirector)
    );
    assert_eq!(
        parse_admin_role("City-Wide Athletic Director"),
        Some(CoachRole::AthleticDirector)
    );
    assert_eq!(
        parse_admin_role("Activities Director"),
        Some(CoachRole::AthleticDirector)
    );
    for office in [
        "AD Admin Assistant",
        "Assistant Athletic Director",
        "Athletic Director Secretary",
        "Athletic Trainer",
        "Principal",
        "Superintendent",
        "Business Manager",
        "",
    ] {
        assert_eq!(parse_admin_role(office), None, "office role {office:?}");
    }
}

#[test]
fn office_staff_are_never_imported_as_coaches_or_directors() -> TestResult {
    let gets = extract_for("135", SCHOOL_GET)?;
    let names: Vec<&str> = gets
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    for dropped in ["Sheryl Byom", "Michele Butler", "Jamie Oliver"] {
        check!(
            !names.contains(&dropped),
            "non-director office row imported as a coach or AD: {dropped} in {names:?}"
        );
    }
    check!(gets
        .skipped_admin_roles
        .iter()
        .any(|role| role == "AD Admin Assistant"));
    check!(gets
        .skipped_admin_roles
        .iter()
        .any(|role| role == "Superintendent"));
    check!(gets
        .skipped_admin_roles
        .iter()
        .any(|role| role == "Principal"));

    let directors: Vec<&CanonicalCoach> = gets
        .coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .collect();
    check!(eq; directors.len(), 1, "exactly one AD row");
    check!(eq; directors[0].name, "Jake Perner");
    check!(eq;
        directors[0].professional_email.as_deref(),
        Some("jakeperner@getschools.k12.wi.us")
    );
    Ok(())
}

#[test]
fn school_without_coach_rows_yields_no_coaching_rows() -> TestResult {
    let extract = extract_for("5151", SCHOOL_SAILS)?;
    check!(eq; extract.school.name, "S.A.I.L.S. CHARTER");
    check!(eq; extract.school.city.as_deref(), Some("Sparta"));
    check!(eq; extract.school.source_identities[0].id, "5151");
    check!(eq; extract.school.classification, None);
    check!(eq; extract.coaches.len(), 2, "two directors, zero coach rows");
    check!(extract
        .coaches
        .iter()
        .all(|coach| coach.role == CoachRole::AthleticDirector));
    check!(eq; extract.skipped_coach_rows, 0);
    let names: Vec<&str> = extract
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    check!(eq; names, vec!["John Blaha", "Adam Dow"]);
    Ok(())
}

#[test]
fn empty_or_malformed_payload_yields_zero_rows() {
    for payload in [
        "",
        "null",
        "   ",
        "<html></html>",
        "{\"error\":\"not found\"}",
        "<table id=\"tblSchools\">",
    ] {
        assert_eq!(
            parse_school_page(payload),
            SchoolPage::default(),
            "payload {payload:?}"
        );
        assert!(
            parse_directory_letter(payload).is_empty(),
            "payload {payload:?}"
        );
        assert_eq!(parse_enrollment(payload), None);
        assert!(decode_cfemail(payload).is_none());
    }
    assert!(decode_cfemail("").is_none());
    assert!(decode_cfemail("abc").is_none());
    assert!(decode_cfemail("zzzz").is_none());
    assert!(decode_cfemail("00010203").is_none());

    let nameless = IndexEntry {
        org_id: "9999".to_string(),
        ..IndexEntry::default()
    };
    assert!(school_entities(&nameless, &SchoolPage::default(), OBSERVED_ON).is_none());
}

#[test]
fn cfemail_decoding_matches_published_addresses() {
    assert_eq!(
        decode_cfemail("7f1514111e0f121613131a0d3f1e1d1d100b0c19100d1b51144e4d510816510a0c")
            .as_deref(),
        Some("jknapmiller@abbotsford.k12.wi.us")
    );
    assert_eq!(
        decode_cfemail("f0919c9182839f9eb09192929f8483969f8294de9bc1c2de8799de8583").as_deref(),
        Some("alarson@abbotsford.k12.wi.us")
    );
}

#[test]
fn honorifics_are_stripped_from_person_names() {
    assert_eq!(strip_honorific("Mr. Barry Mink"), "Barry Mink");
    assert_eq!(strip_honorific("Coach Dana Bell"), "Dana Bell");
    assert_eq!(strip_honorific("Dr. Ana Ruiz"), "Ana Ruiz");
    assert_eq!(strip_honorific("JACOB  KNAPMILLER"), "JACOB KNAPMILLER");
    assert_eq!(strip_honorific("   "), "");
}

#[test]
fn measured_email_fill_rate_on_captured_pages() -> TestResult {
    let mut rows = 0usize;
    let mut with_email = 0usize;
    for (org_id, fixture) in [
        ("1", SCHOOL_ABBOTSFORD),
        ("135", SCHOOL_GET),
        ("5151", SCHOOL_SAILS),
    ] {
        let extract = extract_for(org_id, fixture)?;
        rows = rows.saturating_add(extract.coaches.len());
        with_email = with_email.saturating_add(
            extract
                .coaches
                .iter()
                .filter(|coach| {
                    coach.professional_email.is_some() || coach.personal_email.is_some()
                })
                .count(),
        );
    }
    check!(eq;
        (with_email, rows),
        (11, 11),
        "captured pages publish an address for every emitted AD/coach row"
    );
    check!(
        with_email > 0,
        "the WIAA directory does publish coach addresses"
    );
    Ok(())
}
