use super::*;
use census_domain::UsJurisdiction;

use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};

/// Verbatim byte range of the 2026-09-19 capture `wi-list-A.html`
/// (`GET https://schools.wiaawi.org/Directory/School/DirectoryLetter?LetterBtn=A`, HTTP 200,
/// 127,772 bytes): the "Showing 39 schools…" banner plus the `#tblSchools` header and its first
/// six data rows, unmodified.
const INDEX_A: &str = include_str!("../../tests/fixtures/wiaa/directory_letter_a.html");

/// Verbatim byte range 109,601-148,863 of the capture `wi-school-1.html`
/// (`GET https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1`, HTTP 200,
/// 179,419 bytes): the jumbotron, the identity block, the website buttons, `#tblAdminList` and
/// `#tblCoachList`.
const SCHOOL_ABBOTSFORD: &str =
    include_str!("../../tests/fixtures/wiaa/school_org1_abbotsford.html");

/// Verbatim byte range 109,601-163,662 of the capture `wi-school-135.html` (`GET …?orgID=135`,
/// HTTP 200, 200,357 bytes). Carries an `AD Admin Assistant` row that must never be imported.
const SCHOOL_GET: &str =
    include_str!("../../tests/fixtures/wiaa/school_org135_gale_ettrick_trempealeau.html");

/// Verbatim byte range 109,601-128,994 of the capture `wi-school-5151.html` (`GET …?orgID=5151`,
/// HTTP 200, 149,086 bytes). Two directors and zero coach rows.
const SCHOOL_SAILS: &str =
    include_str!("../../tests/fixtures/wiaa/school_org5151_sails_charter.html");

const OBSERVED_ON: &str = "2026-09-20";

fn entry_for(org_id: &str) -> IndexEntry {
    parse_directory_letter(INDEX_A)
        .into_iter()
        .find(|entry| entry.org_id == org_id)
        .unwrap_or_else(|| IndexEntry {
            org_id: org_id.to_string(),
            ..IndexEntry::default()
        })
}

fn extract_for(org_id: &str, fixture: &str) -> SchoolExtract {
    let entry = entry_for(org_id);
    let page = parse_school_page(fixture);
    school_entities(&entry, &page, OBSERVED_ON).expect("fixture page yields entities")
}

#[test]
fn index_parsing_yields_org_ids_level_and_city() {
    let entries = parse_directory_letter(INDEX_A);
    assert_eq!(entries.len(), 6, "fixture keeps six real index rows");
    assert_eq!(entries[0].org_id, "1");
    assert_eq!(entries[0].name, "ABBOTSFORD");
    assert_eq!(entries[0].level, "High School");
    assert_eq!(entries[0].city, "Abbotsford");
    assert_eq!(
        entries[0].page_url(),
        format!("{HOST}{SCHOOL_PATH}?orgID=1")
    );
    // The `title` attribute is the untruncated name; the visible <h5> is CSS-truncated.
    let last = entries.last().expect("six entries");
    assert_eq!(last.name, "ADVANCED LEARNING ACADEMY OF WISCONSIN CHARTER");
    assert_eq!(last.level, "High School");
    assert!(entries.iter().any(|entry| entry.level == "Middle School"));
    let mut ids: Vec<&str> = entries.iter().map(|entry| entry.org_id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), entries.len(), "orgIDs are unique");
    assert!(entries.iter().all(|entry| !entry.org_id.is_empty()));
}

#[test]
fn empty_index_fragment_yields_no_rows() {
    assert!(parse_directory_letter("").is_empty());
    assert!(parse_directory_letter("null").is_empty());
    // Shape of the `LetterBtn=-1` fragment: HTTP 200, 3,077 bytes, no school table.
    assert!(parse_directory_letter("<div class=\"alert\"></div>").is_empty());
}

#[test]
fn school_page_yields_name_city_conference_ad_and_identity() {
    let page = parse_school_page(SCHOOL_ABBOTSFORD);
    assert_eq!(page.name, "Abbotsford");
    assert_eq!(page.city.as_deref(), Some("Abbotsford"));
    assert_eq!(page.conference.as_deref(), Some("Marawood"));
    assert_eq!(page.level.as_deref(), Some("High School"));
    assert_eq!(page.enrollment, Some(214));
    assert_eq!(
        page.website.as_deref(),
        Some("http://www.abbotsford.k12.wi.us")
    );

    let extract = extract_for("1", SCHOOL_ABBOTSFORD);
    assert_eq!(extract.school.name, "Abbotsford");
    assert_eq!(extract.school.state, Some(UsJurisdiction::Wisconsin));
    assert_eq!(extract.school.city.as_deref(), Some("Abbotsford"));
    assert_eq!(extract.school.association.as_deref(), Some("wiaa"));
    assert_eq!(extract.school.classification.as_deref(), Some("Marawood"));
    assert_eq!(extract.school.enrollment, Some(214));
    assert_eq!(
        extract.school.school_website.as_deref(),
        Some("http://www.abbotsford.k12.wi.us")
    );
    // Canonical id is deterministic from state + normalized name, so another provider that saw
    // Abbotsford mints the same school.
    assert_eq!(
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
        .expect("one provider identity");
    assert_eq!(
        identity.namespace,
        SourceNamespace::AssociationSchool {
            association: "wiaa".to_string()
        }
    );
    assert_eq!(identity.id, "1");
    assert_eq!(
        identity.url.as_deref(),
        Some("https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1")
    );
    assert_eq!(extract.school.evidence.len(), 1);
    assert_eq!(extract.school.evidence[0].observed_on, OBSERVED_ON);
    assert_eq!(extract.school.evidence[0].source.id, SOURCE_ID);
    assert!(extract.school.evidence[0]
        .source
        .url
        .as_deref()
        .is_some_and(|url| url.contains("orgID=1")));

    // Role and sport labels are scraped out of `<label>` elements; markup must never survive
    // into the parsed label (the role string feeds the director/coach classifier).
    assert!(!page.admins.is_empty(), "Abbotsford publishes office rows");
    for admin in &page.admins {
        assert!(
            !admin.role.contains('<') && !admin.role.contains('>'),
            "markup in admin role {:?}",
            admin.role
        );
    }
    for coach in &page.coaches {
        assert!(
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
        .expect("Abbotsford publishes an athletic director");
    assert_eq!(ad.name, "Alex Larson");
    assert_eq!(ad.sport, None, "an AD is school-wide, never sport-bound");
    assert_eq!(ad.gender, Gender::Mixed);
    assert_eq!(
        ad.professional_email.as_deref(),
        Some("alarson@abbotsford.k12.wi.us")
    );
    assert_eq!(ad.evidence.len(), 1);
    assert_eq!(ad.evidence[0].observed_on, OBSERVED_ON);
}

#[test]
fn coach_rows_map_to_sport_and_gender() {
    let extract = extract_for("1", SCHOOL_ABBOTSFORD);
    let find = |name: &str| -> Vec<&CanonicalCoach> {
        extract
            .coaches
            .iter()
            .filter(|coach| coach.name == name)
            .collect()
    };

    let knapmiller = find("JACOB KNAPMILLER");
    assert_eq!(knapmiller.len(), 2, "same person, two sport-gender rows");
    let mut pairs: Vec<(Sport, Gender)> = knapmiller
        .iter()
        .map(|coach| (coach.sport.expect("sport-bound"), coach.gender))
        .collect();
    pairs.sort_unstable();
    assert_eq!(
        pairs,
        vec![
            (Sport::OutdoorTrack, Gender::Boys),
            (Sport::OutdoorTrack, Gender::Girls)
        ]
    );
    assert!(knapmiller
        .iter()
        .all(|coach| coach.role == CoachRole::HeadCoach));
    assert!(knapmiller.iter().all(|coach| {
        coach.professional_email.as_deref() == Some("jknapmiller@abbotsford.k12.wi.us")
    }));

    let novak = find("Dillon Novak");
    assert_eq!(novak.len(), 1);
    assert_eq!(novak[0].sport, Some(Sport::CrossCountry));
    assert_eq!(novak[0].gender, Gender::Girls);
    assert_eq!(novak[0].role, CoachRole::HeadCoach);
    assert_eq!(
        novak[0].professional_email.as_deref(),
        Some("dnovak@abbotsford.k12.wi.us")
    );

    // Only TF/XC rows survive: Abbotsford publishes 14 coach rows, of which 3 are TF/XC.
    assert_eq!(
        extract.coaches.len(),
        4,
        "one AD + three TF/XC head coaches"
    );
    assert_eq!(extract.skipped_coach_rows, 11);
    for coach in &extract.coaches {
        if coach.role == CoachRole::AthleticDirector {
            continue;
        }
        assert!(matches!(
            coach.sport,
            Some(Sport::OutdoorTrack | Sport::CrossCountry)
        ));
    }

    // A second school: G-E-T's Paula Gold holds three TF/XC roles, all three must survive.
    let gets = extract_for("135", SCHOOL_GET);
    let gold: Vec<&CanonicalCoach> = gets
        .coaches
        .iter()
        .filter(|coach| coach.name == "Paula Gold")
        .collect();
    let mut gold_pairs: Vec<(String, Gender)> = gold
        .iter()
        .map(|coach| {
            let sport = coach.sport.expect("sport-bound");
            (format!("{sport:?}"), coach.gender)
        })
        .collect();
    gold_pairs.sort();
    assert_eq!(
        gold_pairs,
        vec![
            ("CrossCountry".to_string(), Gender::Boys),
            ("CrossCountry".to_string(), Gender::Girls),
            ("OutdoorTrack".to_string(), Gender::Girls),
        ]
    );
    assert_eq!(gets.coaches.len(), 5, "one AD + four TF/XC head coaches");
    assert_eq!(gets.skipped_coach_rows, 19);
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
    // Real label from the captured sample (orgID 219 Madison East): a district-level director
    // published inside one school's administration table. It is a role-published director for
    // that school, so it is kept; the assistant AD row beside it is not.
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
fn office_staff_are_never_imported_as_coaches_or_directors() {
    let gets = extract_for("135", SCHOOL_GET);
    let names: Vec<&str> = gets
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    // Real rows of the captured page: an office assistant inside the AD's table, plus the
    // building administration.
    for dropped in ["Sheryl Byom", "Michele Butler", "Jamie Oliver"] {
        assert!(
            !names.contains(&dropped),
            "non-director office row imported as a coach or AD: {dropped} in {names:?}"
        );
    }
    assert!(gets
        .skipped_admin_roles
        .iter()
        .any(|role| role == "AD Admin Assistant"));
    assert!(gets
        .skipped_admin_roles
        .iter()
        .any(|role| role == "Superintendent"));
    assert!(gets
        .skipped_admin_roles
        .iter()
        .any(|role| role == "Principal"));

    let directors: Vec<&CanonicalCoach> = gets
        .coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .collect();
    assert_eq!(directors.len(), 1, "exactly one AD row");
    assert_eq!(directors[0].name, "Jake Perner");
    assert_eq!(
        directors[0].professional_email.as_deref(),
        Some("jakeperner@getschools.k12.wi.us")
    );
}

#[test]
fn school_without_coach_rows_yields_no_coaching_rows() {
    let extract = extract_for("5151", SCHOOL_SAILS);
    assert_eq!(extract.school.name, "S.A.I.L.S. CHARTER");
    assert_eq!(extract.school.city.as_deref(), Some("Sparta"));
    assert_eq!(extract.school.source_identities[0].id, "5151");
    // WIAA prints "N/A" for this school's conference; a placeholder must not become a field.
    assert_eq!(extract.school.classification, None);
    assert_eq!(extract.coaches.len(), 2, "two directors, zero coach rows");
    assert!(extract
        .coaches
        .iter()
        .all(|coach| coach.role == CoachRole::AthleticDirector));
    assert_eq!(extract.skipped_coach_rows, 0);
    let names: Vec<&str> = extract
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(names, vec!["John Blaha", "Adam Dow"]);
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

    // An index row with an orgID but no name mints nothing.
    let nameless = IndexEntry {
        org_id: "9999".to_string(),
        ..IndexEntry::default()
    };
    assert!(school_entities(&nameless, &SchoolPage::default(), OBSERVED_ON).is_none());
}

#[test]
fn cfemail_decoding_matches_published_addresses() {
    // Verbatim `data-cfemail` payloads from the Abbotsford captures.
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

/// Measured, not assumed: the fill rate below is what the captured pages actually publish.
#[test]
fn measured_email_fill_rate_on_captured_pages() {
    let mut rows = 0usize;
    let mut with_email = 0usize;
    for (org_id, fixture) in [
        ("1", SCHOOL_ABBOTSFORD),
        ("135", SCHOOL_GET),
        ("5151", SCHOOL_SAILS),
    ] {
        let extract = extract_for(org_id, fixture);
        rows = rows.saturating_add(extract.coaches.len());
        with_email = with_email.saturating_add(
            extract
                .coaches
                .iter()
                .filter(|coach| coach.professional_email.is_some())
                .count(),
        );
    }
    assert_eq!(
        (with_email, rows),
        (11, 11),
        "captured pages publish an address for every emitted AD/coach row"
    );
    assert!(
        with_email > 0,
        "the WIAA directory does publish coach addresses"
    );
}
