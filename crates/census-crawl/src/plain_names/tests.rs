use super::*;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

/// Fixture provenance — every file is a real capture (or, for `ND_PAGE_NO_AD`, one documented
/// deletion from one).
///
/// * `ND_INDEX` — `GET https://ndhsaa.com/schools`, HTTP 200, capture
///   `tools/a29-coach/nd-schools.html` (2026-09-19 23:23 CDT), 169 member-school anchors.
/// * `ND_PAGE` — `GET https://ndhsaa.com/schools/1045/west-fargo-sheyenne`, HTTP 200, capture
///   `tools/a29-coach/nd-1045-west-fargo-sheyenne.html` (2026-09-19 23:24 CDT).
/// * `ND_PAGE_NO_AD` — `GET https://ndhsaa.com/schools/1378/mandan-classical-academy`, live GET
///   HTTP 200 2026-09-20T14:23:43Z, minus the one `Athletic Director: …` paragraph: the provider
///   publishes an AD for every sampled member school (33/33 in research reports 25 & 37, 4/4 in
///   this session's live probes), so the no-AD shape is reproduced by deleting that line.
/// * `NSAA_PAGE` — the directory screen's bulk POST (`session= `, `school=View all schools`,
///   `submit=See School Info`), live HTTP 200 1,085,584 B 2026-09-20T14:24:47Z (byte-identical to
///   `tools/ne/nsaa_directory_all_2026-09-19.html`), sliced to its first 8 school blocks. The same
///   `NsaaSchool` markup appears one school at a time in `NSAA_SCHOOL_GET`.
/// * `NSAA_FORM` — `GET https://secure.nsaahome.org/nsaaforms/direxportscreen.php`, live HTTP 200
///   11,585 B 2026-09-20T14:36:10Z: the request form plus the 314 `<option>` entries (312 schools
///   + a disabled placeholder + the "View all schools" sentinel).
/// * `NSAA_SCHOOL_GET` — `GET …?session=&school=Adams%20Central`, live HTTP 200 15,579 B
///   2026-09-20T14:36:12Z: one school, one `<h1 class="mt-3">` block, the same 34 rows as the
///   bulk block for Adams Central.
const ND_INDEX: &str = include_str!("../../tests/fixtures/plain_names/nd_schools_index.html");
const ND_PAGE: &str = include_str!("../../tests/fixtures/plain_names/nd_school_page.html");
const ND_PAGE_NO_AD: &str =
    include_str!("../../tests/fixtures/plain_names/nd_school_page_no_ad.html");
const NSAA_PAGE: &str = include_str!("../../tests/fixtures/plain_names/nsaa_directory_export.html");
const NSAA_FORM: &str = include_str!("../../tests/fixtures/plain_names/nsaa_directory_form.html");
const NSAA_SCHOOL_GET: &str =
    include_str!("../../tests/fixtures/plain_names/nsaa_school_get_adams_central.html");

const OBSERVED_ON: &str = "2026-09-20";

fn sheyenne() -> NdSchoolRef {
    NdSchoolRef {
        id: "1045".to_string(),
        slug: "west-fargo-sheyenne".to_string(),
    }
}

fn mandan_classical() -> NdSchoolRef {
    NdSchoolRef {
        id: "1378".to_string(),
        slug: "mandan-classical-academy".to_string(),
    }
}

/// The request URL the adapter would have used for a fixture school: evidence always cites the
/// per-school request that produced the row, never the directory form that listed it.
fn url_of(school: &NsaaSchool) -> String {
    nsaa_school_url(&school.name)
}

/// Every string field of an entity, so a test can prove a value cannot leak from *any* field.
fn serialized<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("entity serializes")
}

// ------------------------------ North Dakota ------------------------------

#[test]
fn nd_index_lists_every_member_school() {
    let members = parse_nd_school_refs(ND_INDEX).expect("the ndhsaa index parses");
    assert_eq!(
        members.len(),
        169,
        "the index carries all 169 member schools"
    );

    let ids: HashSet<&str> = members.iter().map(|member| member.id.as_str()).collect();
    assert_eq!(ids.len(), members.len(), "ids are unique after dedupe");
    assert!(ids.contains("1045"));

    let sheyenne = members
        .iter()
        .find(|member| member.id == "1045")
        .expect("West Fargo Sheyenne is a member");
    assert_eq!(sheyenne.slug, "west-fargo-sheyenne");
    assert_eq!(
        sheyenne.url(),
        "https://ndhsaa.com/schools/1045/west-fargo-sheyenne"
    );
}

#[test]
fn nd_index_dedupes_and_ignores_other_links() {
    let html = r#"
            <a href="https://ndhsaa.com/schools/7/bismarck">Bismarck</a>
            <a href="/schools/7/bismarck-high">Bismarck again</a>
            <a href="/schools">All schools</a>
            <a href="https://ndhsaa.com/athletics/track-boys">Track</a>
            <a href="/schools/92/alexander">Alexander</a>
        "#;
    let members = parse_nd_school_refs(html).expect("the ndhsaa index parses");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].slug, "bismarck", "first link per id wins");
    assert_eq!(members[1].id, "92");
}

#[test]
fn nd_school_page_parses_school_metadata() {
    let (school, school_id) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)
        .expect("the ndhsaa page parses")
        .expect("fixture has a heading");

    assert_eq!(school.name, "West Fargo Sheyenne High School");
    assert_eq!(
        school.normalized_name, "west fargo sheyenne",
        "normalize_name drops the `High School` suffix"
    );
    assert_eq!(school.state, Some(UsJurisdiction::NorthDakota));
    assert_eq!(school.association.as_deref(), Some("ndhsaa"));
    assert_eq!(school.city.as_deref(), Some("West Fargo"));
    assert_eq!(
        school.enrollment,
        Some(1399),
        "1,399 students enrolled in 2025"
    );
    assert_eq!(
        school.school_website.as_deref(),
        Some("https://www.west-fargo.k12.nd.us/shs/activities")
    );
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "ndhsaa".to_string()
        }
    );
    assert_eq!(school.source_identities[0].id, "1045");
    assert_eq!(
        school.source_identities[0].url.as_deref(),
        Some("https://ndhsaa.com/schools/1045/west-fargo-sheyenne")
    );
    assert_eq!(school.evidence.len(), 1);
    assert_eq!(school.evidence[0].observed_on, OBSERVED_ON);
    assert_eq!(school.evidence[0].source.id, "ndhsaa");
    assert_eq!(
        school.evidence[0].source.url.as_deref(),
        Some("https://ndhsaa.com/schools/1045/west-fargo-sheyenne")
    );
    assert_eq!(
        school_id,
        CanonicalSchool::mint(
            UsJurisdiction::NorthDakota,
            &school.name,
            &school.normalized_name
        )
    );
    // Identity is the natural key, not the display name: the same school published elsewhere as
    // "West Fargo Sheyenne HS" mints the identical id, so the two observations merge.
    let (_, same_school) = CanonicalSchool::new(
        UsJurisdiction::NorthDakota,
        "West Fargo Sheyenne HS",
        normalize_name("West Fargo Sheyenne HS"),
    );
    assert_eq!(school_id, same_school);
}

#[test]
fn nd_school_page_never_stores_phone_fax_or_address() {
    let (school, _) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)
        .expect("the ndhsaa page parses")
        .expect("school");
    let json = serialized(&school);
    assert!(
        !json.contains("356-2160"),
        "the school phone number is not stored"
    );
    assert!(!json.contains("499-6687"), "the fax number is not stored");
    assert!(
        !json.contains("800 40th Ave E."),
        "the street address is not stored"
    );
}

#[test]
fn nd_ad_coaches_include_ad_and_activities_director_only() {
    let (_, school_id) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)
        .expect("the ndhsaa page parses")
        .expect("school");
    let staff = parse_nd_staff(ND_PAGE).expect("the ndhsaa staff lines parse");
    let coaches = nd_ad_coaches(
        &staff,
        &school_id,
        "https://ndhsaa.com/schools/1045/west-fargo-sheyenne",
        OBSERVED_ON,
    );

    let names: Vec<&str> = coaches.iter().map(|coach| coach.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["Logan Midthun", "James Moe", "Corissa Kolesar"],
        "Athletic Director and Activities Director rows collapse to one entity per person"
    );
    for coach in &coaches {
        assert_eq!(coach.role, CoachRole::AthleticDirector);
        assert_eq!(coach.sport, None, "directors are school-wide roles");
        assert_eq!(coach.gender, Gender::Mixed);
        assert_eq!(coach.professional_email, None);
        assert_eq!(coach.phone, None);
        assert_eq!(coach.evidence.len(), 1);
        assert_eq!(coach.evidence[0].observed_on, OBSERVED_ON);
    }

    // The office roles are on the page and were parsed as staff lines — they are simply never
    // promoted to coaches, however senior the person is.
    let labels: Vec<&str> = staff.iter().map(|role| role.label.as_str()).collect();
    for office in [
        "Superintendent",
        "Assistant Superintendent",
        "Principal",
        "Vice/Assistant Principal",
        "Business Manager",
        "Tech Director",
    ] {
        assert!(
            labels.contains(&office),
            "{office} is a published staff line"
        );
    }
    for (label, person) in [
        ("Superintendent", "Beth Slette"),
        ("Assistant Superintendent", "Vincent Williams"),
        ("Principal", "Ryan Salisbury"),
        ("Vice/Assistant Principal", "Ryan Bodell"),
        ("Business Manager", "Levi Bachmeier"),
        ("Tech Director", "Ed Mitchell"),
    ] {
        assert!(
            !names.contains(&person),
            "{person} ({label}) must never be emitted as a coach or director"
        );
        assert!(
            parse_nd_role(label).is_none(),
            "{label} is not a director role"
        );
    }
}

#[test]
fn nd_page_without_ad_yields_no_director_rows() {
    let (school, school_id) = parse_nd_school_page(ND_PAGE_NO_AD, &mandan_classical(), OBSERVED_ON)
        .expect("the ndhsaa page parses")
        .expect("the page still yields a school");
    assert_eq!(school.name, "Mandan Classical Academy");
    assert_eq!(school.city.as_deref(), Some("Mandan"));
    assert_eq!(school.association.as_deref(), Some("ndhsaa"));

    let staff = parse_nd_staff(ND_PAGE_NO_AD).expect("the ndhsaa staff lines parse");
    assert!(
        !staff
            .iter()
            .any(|role| role.label.to_ascii_lowercase().contains("director")),
        "fixture really carries no director line"
    );
    assert!(
        staff
            .iter()
            .any(|role| role.label == "Superintendent" && role.name == "Thomas Hoopes"),
        "the superintendent is published on this page"
    );

    let coaches = nd_ad_coaches(
        &staff,
        &school_id,
        "https://ndhsaa.com/schools/1378/mandan-classical-academy",
        OBSERVED_ON,
    );
    assert!(
        coaches.is_empty(),
        "a superintendent is not an athletic director, however senior"
    );
}

#[test]
fn nd_offering_rows_and_coop_annotations_parse() {
    let offerings = parse_nd_offerings(ND_PAGE).expect("the ndhsaa offering rows parse");
    assert_eq!(offerings.len(), 28, "one row per published offering");

    let cross_country = offerings
        .iter()
        .find(|offering| offering.label == "Boys' Cross Country")
        .expect("Boys' Cross Country is offered");
    assert_eq!(cross_country.coaches, vec!["Troy Thorson", "Jared Slinde"]);
    assert_eq!(cross_country.co_op, None);

    let hockey = offerings
        .iter()
        .find(|offering| offering.label == "Boys' Ice Hockey")
        .expect("Boys' Ice Hockey is offered");
    assert_eq!(
        hockey.co_op.as_deref(),
        Some("West Fargo Sheyenne"),
        "the co-op annotation is recorded, not left inside the sport label"
    );

    let blank = offerings
        .iter()
        .find(|offering| offering.label == "Debate")
        .expect("Debate is offered");
    assert!(
        blank.coaches.is_empty(),
        "a blank coach cell yields no names"
    );
}

#[test]
fn nd_sport_labels_map_to_sport_and_gender() {
    assert_eq!(
        parse_nd_sport("Boys' Cross Country"),
        Some((Sport::CrossCountry, Gender::Boys))
    );
    assert_eq!(
        parse_nd_sport("Girls' Cross Country"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        parse_nd_sport("Boys' Track and Field"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        parse_nd_sport("Girls' Track and Field"),
        Some((Sport::OutdoorTrack, Gender::Girls))
    );
    assert_eq!(
        parse_nd_sport("Boys' Indoor Track"),
        Some((Sport::IndoorTrack, Gender::Boys))
    );
    for other in [
        "Cheer - Boys' Basketball",
        "Volleyball",
        "Music - Vocal",
        "Student Congress",
        "Girls' Wrestling",
    ] {
        assert_eq!(
            parse_nd_sport(other),
            None,
            "{other} is not a TF/XC offering"
        );
    }
}

#[test]
fn nd_sport_coaches_are_names_only() {
    let (_, school_id) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)
        .expect("the ndhsaa page parses")
        .expect("school");
    let offerings = parse_nd_offerings(ND_PAGE).expect("the ndhsaa offering rows parse");
    let coaches = nd_sport_coaches(
        &offerings,
        &school_id,
        "https://ndhsaa.com/schools/1045/west-fargo-sheyenne",
        OBSERVED_ON,
    );

    // 4 TF/XC offerings: 2+2 XC names, 1 boys TF name, 1 girls TF name = 6 entities.
    assert_eq!(coaches.len(), 6);
    for coach in &coaches {
        assert_eq!(
            coach.role,
            CoachRole::Unknown,
            "NDHSAA publishes no head/assistant split"
        );
        assert!(coach.sport.is_some());
        assert_eq!(coach.professional_email, None);
        assert_eq!(coach.phone, None);
    }
    let girls_track = coaches
        .iter()
        .find(|coach| coach.sport == Some(Sport::OutdoorTrack) && coach.gender == Gender::Girls)
        .expect("girls track coach");
    assert_eq!(girls_track.name, "Jaime Watson");
    assert!(
        coaches
            .iter()
            .all(|coach| { !coach.evidence.is_empty() && coach.evidence[0].source.id == "ndhsaa" }),
        "every coach cites the page it came from"
    );
    // Non-TF/XC offerings contribute nothing.
    assert!(!coaches.iter().any(|coach| coach.name == "Tim Brandt"));
}

#[test]
fn nd_fixture_reports_its_own_fill_rate() {
    let offerings = parse_nd_offerings(ND_PAGE).expect("the ndhsaa offering rows parse");
    let slots: Vec<&NdOffering> = offerings
        .iter()
        .filter(|offering| parse_nd_sport(&offering.label).is_some())
        .collect();
    let named = slots
        .iter()
        .filter(|offering| !offering.coaches.is_empty())
        .count();
    assert_eq!(
        slots.len(),
        4,
        "the fixture page offers boys/girls XC and track"
    );
    assert_eq!(
        named, 4,
        "all four TF/XC coach slots are named on this page"
    );
}

#[test]
fn nd_malformed_input_yields_no_rows() {
    assert!(parse_nd_school_refs("<html>no links here</html>")
        .expect("the ndhsaa index parses")
        .is_empty());
    assert!(parse_nd_school_refs("")
        .expect("the ndhsaa index parses")
        .is_empty());
    assert!(parse_nd_school_page(
        "<html><body>Nothing</body></html>",
        &sheyenne(),
        OBSERVED_ON
    )
    .expect("the ndhsaa page parses")
    .is_none());
    assert!(parse_nd_staff("not html at all")
        .expect("the ndhsaa staff lines parse")
        .is_empty());
    assert!(
        parse_nd_offerings("<table><tr><td>only one cell</td></tr></table>")
            .expect("the ndhsaa offering rows parse")
            .is_empty()
    );
    // An empty heading is not a school name.
    assert!(parse_nd_school_page(
        "<h1>   </h1><p>Address: x, Fargo, ND 58102</p>",
        &sheyenne(),
        OBSERVED_ON
    )
    .expect("the ndhsaa page parses")
    .is_none());
}

// ------------------------------ Nebraska ------------------------------

#[test]
fn nsaa_form_option_list_yields_the_member_schools() {
    let names = parse_nsaa_school_names(NSAA_FORM).expect("the nsaa option list parses");
    assert_eq!(names.len(), 312, "the form lists every member school");
    assert_eq!(names[0], "Adams Central");
    assert_eq!(names[1], "Ainsworth");
    assert_eq!(names[311], "Yutan");
    assert!(
        !names.iter().any(|name| name == NSAA_ALL_SCHOOLS),
        "the bulk sentinel is not a school"
    );
    assert!(
        !names.iter().any(|name| name.contains("Select a school")),
        "the disabled placeholder is not a school"
    );
    assert_eq!(
        names
            .iter()
            .filter(|name| name.as_str() == "Adams Central")
            .count(),
        1,
        "names are unique"
    );

    // The form's option names are exactly the names the bulk response uses for its blocks.
    let bulk: Vec<String> = parse_nsaa_directory(NSAA_PAGE)
        .expect("the nsaa directory parses")
        .into_iter()
        .map(|school| school.name)
        .collect();
    for name in &bulk {
        assert!(
            names.contains(name),
            "{name} is an option and a block heading"
        );
    }

    // Malformed and empty payloads yield no rows rather than panicking.
    assert!(parse_nsaa_school_names("")
        .expect("the nsaa option list parses")
        .is_empty());
    assert!(parse_nsaa_school_names("<select></select>")
        .expect("the nsaa option list parses")
        .is_empty());
    assert!(parse_nsaa_school_names("<option></option>")
        .expect("the nsaa option list parses")
        .is_empty());
    assert!(
        parse_nsaa_school_names("<option disabled>Select a school to view...</option>")
            .expect("the nsaa option list parses")
            .is_empty()
    );
}

#[test]
fn nsaa_school_url_round_trips_the_published_name() {
    assert_eq!(
            nsaa_school_url("Adams Central"),
            "https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Adams+Central",
            "the `+` form is what `byte_serialize` emits; the server decoded it to the same 15,579-byte \
             page as the `%20` form (live 2026-09-20T14:39:13Z, HTTP 200, byte-identical)"
        );
    assert_eq!(
        nsaa_school_url("Anselmo-Merna"),
        "https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Anselmo-Merna",
        "hyphens are safe and stay literal"
    );

    // Every published name must survive the encoding: parse the URL back and compare.
    for name in parse_nsaa_school_names(NSAA_FORM).expect("the nsaa option list parses") {
        let url = url::Url::parse(&nsaa_school_url(&name)).expect("valid url");
        let decoded = url
            .query_pairs()
            .find(|(key, _)| key == "school")
            .map(|(_, value)| value.into_owned())
            .expect("school query parameter");
        assert_eq!(decoded, name, "round-trip for {name}");
    }
}

#[test]
fn nsaa_single_school_page_matches_the_bulk_block() {
    let single = parse_nsaa_directory(NSAA_SCHOOL_GET).expect("the nsaa directory parses");
    assert_eq!(single.len(), 1, "one school per single-school response");
    let entry = &single[0];
    assert_eq!(entry.name, "Adams Central");
    assert_eq!(entry.roles.len(), 34);
    assert_eq!(entry.city.as_deref(), Some("Hastings"));
    assert_eq!(entry.enrollment, Some(215));
    assert_eq!(
        entry.homepage.as_deref(),
        Some("http://www.adamscentral.us/")
    );

    // The same school from the bulk capture produces identical entities.
    let bulk = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let bulk_adams = bulk
        .iter()
        .find(|school| school.name == "Adams Central")
        .expect("Adams Central in the bulk slice");
    assert_eq!(entry.roles, bulk_adams.roles);

    let url = nsaa_school_url("Adams Central");
    let (school, school_id) = parse_nsaa_school(entry, &url, OBSERVED_ON);
    let coaches =
        nsaa_coaches(entry, &school_id, &url, OBSERVED_ON).expect("the nsaa coach rows parse");
    let (_, bulk_id) = parse_nsaa_school(bulk_adams, &url, OBSERVED_ON);
    assert_eq!(school_id, bulk_id, "one canonical id either way");
    assert_eq!(coaches.len(), 6);
    assert_eq!(
        school.evidence[0].source.url.as_deref(),
        Some(nsaa_school_url("Adams Central").as_str()),
        "evidence cites the request URL that produced the row"
    );
}

#[test]
fn nsaa_directory_parses_every_school_block() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let names: Vec<&str> = schools.iter().map(|school| school.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Adams Central",
            "Ainsworth",
            "Allen",
            "Alliance",
            "Alma",
            "Amherst",
            "Anselmo-Merna",
            "Ansley"
        ]
    );

    let adams = &schools[0];
    assert_eq!(adams.city.as_deref(), Some("Hastings"));
    assert_eq!(adams.enrollment, Some(215));
    assert_eq!(
        adams.homepage.as_deref(),
        Some("http://www.adamscentral.us/")
    );
    assert_eq!(
        adams.roles.len(),
        34,
        "one row per published staff/coach line"
    );

    let coop_rows = schools
        .iter()
        .flat_map(|school| school.roles.iter())
        .filter(|role| role.co_op)
        .count();
    assert_eq!(
        coop_rows, 32,
        "rows the directory highlights as co-op-shared carry `class='table-info'`"
    );
    assert_eq!(
        adams.roles.iter().filter(|role| role.co_op).count(),
        3,
        "Adams Central's co-op rows: Softball and the two Swimming placeholders"
    );
    let allen = schools
        .iter()
        .find(|school| school.name == "Allen")
        .expect("Allen in the fixture");
    assert_eq!(allen.roles.iter().filter(|role| role.co_op).count(), 13);
    let ansley = schools
        .iter()
        .find(|school| school.name == "Ansley")
        .expect("Ansley in the fixture");
    assert_eq!(ansley.roles.iter().filter(|role| role.co_op).count(), 10);
}

#[test]
fn nsaa_sport_rows_map_to_head_coach_sport_and_gender() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let adams = &schools[0];
    assert_eq!(
        parse_nsaa_row("Cross-Country (Boys)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::CrossCountry,
            gender: Gender::Boys
        })
    );
    assert_eq!(
        parse_nsaa_row("Cross-Country (Girls)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::CrossCountry,
            gender: Gender::Girls
        })
    );
    assert_eq!(
        parse_nsaa_row("Track & Field (Boys)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys
        })
    );
    assert_eq!(
        parse_nsaa_row("Track & Field (Girls)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::OutdoorTrack,
            gender: Gender::Girls
        })
    );
    assert_eq!(
        parse_nsaa_row("Unified Track & Field"),
        None,
        "a distinct NSAA activity"
    );
    assert_eq!(parse_nsaa_row("Strength Coach"), None);
    assert_eq!(parse_nsaa_row("Volleyball"), None);

    let (_, school_id) = parse_nsaa_school(adams, &url_of(adams), OBSERVED_ON);
    let coaches = nsaa_coaches(adams, &school_id, &url_of(adams), OBSERVED_ON)
        .expect("the nsaa coach rows parse");
    let track_boys = coaches
        .iter()
        .find(|coach| {
            coach.sport == Some(Sport::OutdoorTrack)
                && coach.gender == Gender::Boys
                && coach.name != "Toni Fowler"
        })
        .expect("boys track coach");
    assert_eq!(track_boys.name, "Zeb Noyd");
    assert_eq!(track_boys.role, CoachRole::HeadCoach);
    let xc_boys = coaches
        .iter()
        .find(|coach| coach.sport == Some(Sport::CrossCountry) && coach.gender == Gender::Boys)
        .expect("boys XC coach");
    assert_eq!(xc_boys.name, "Toni Fowler");
    assert_eq!(xc_boys.role, CoachRole::HeadCoach);
}

#[test]
fn nsaa_multi_name_cells_split_into_one_entity_per_person() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let ainsworth = schools
        .iter()
        .find(|school| school.name == "Ainsworth")
        .expect("Ainsworth in the fixture");
    let (_, school_id) = parse_nsaa_school(ainsworth, &url_of(ainsworth), OBSERVED_ON);
    let coaches = nsaa_coaches(ainsworth, &school_id, &url_of(ainsworth), OBSERVED_ON)
        .expect("the nsaa coach rows parse");

    let xc: Vec<&str> = coaches
        .iter()
        .filter(|coach| coach.sport == Some(Sport::CrossCountry))
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(
        xc,
        vec![
            "Trey Schlueter",
            "Katie Winters",
            "Trey Schlueter",
            "Katie Winters"
        ],
        "`Trey Schlueter/Katie Winters` is two people, not one name field"
    );
    assert!(!xc.iter().any(|name| name.contains('/')));
}

#[test]
fn nsaa_director_rows_map_to_athletic_director() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let adams = &schools[0];
    let (school, school_id) = parse_nsaa_school(adams, &url_of(adams), OBSERVED_ON);
    assert_eq!(school.state, Some(UsJurisdiction::Nebraska));
    assert_eq!(school.association.as_deref(), Some("nsaa"));
    assert_eq!(school.city.as_deref(), Some("Hastings"));
    assert_eq!(school.enrollment, Some(215));
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "nsaa".to_string()
        }
    );
    assert_eq!(
        school.source_identities[0].id, "Adams Central",
        "NSAA publishes no numeric id, so the published name is the provider key"
    );

    let coaches = nsaa_coaches(adams, &school_id, &url_of(adams), OBSERVED_ON)
        .expect("the nsaa coach rows parse");
    let directors: Vec<&str> = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(
        directors,
        vec!["Alan Frank", "Aub Boucher"],
        "Activities Director + Athletic Director collapse, the assistant director is an AD row"
    );
    for coach in coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
    {
        assert_eq!(coach.sport, None);
        assert_eq!(coach.gender, Gender::Mixed);
    }
}

#[test]
fn nsaa_office_roles_are_never_emitted() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    // Every one of these people is published in an office row in the fixture and appears in no
    // coach/AD row anywhere in it.
    let office_people = [
        "Shawn Scott",           // Superintendent, Adams Central
        "Scott Harrington",      // Principal, Adams Central
        "Mattison Tinant",       // AD Secretary, Adams Central
        "Dave Johnson",          // Board President, Adams Central
        "Becky Fisher",          // Guidance Counselor, Adams Central
        "Sean Vonderfecht",      // Trainer, Adams Central
        "Dale Hafer",            // Superintendent, Ainsworth
        "Kari Painter",          // AD Secretary, Ainsworth
        "Brad Wilkins",          // Board President, Ainsworth
        "Jerry Bockman",         // Trainer, Ainsworth
        "Mike Pattee",           // Superintendent, Allen
        "Chris Blohm",           // Principal, Allen
        "Becky Stapleton",       // AD Secretary, Allen
        "Jason Olesen",          // Board President, Allen
        "Kim Jonas",             // Superintendent, Ansley
        "Chrissy Slingsby",      // AD Secretary, Ansley
        "Roger Thomsen",         // Superintendent *and* Principal, Amherst
        "Carlene Abbott",        // AD Secretary, Amherst
        "Bobbi Sorensen",        // Guidance Counselor, Amherst
        "Aaron Klingelhoefer",   // Trainer, Amherst
        "Lloyd McIntyre", // Superintendent, Anselmo-Merna (also coaches Golf, not a census sport)
        "Molli Miller",   // Guidance Counselor, Anselmo-Merna
        "Dr. Troy Unzicker", // Superintendent, Alliance
        "Marissa Rotness", // AD Secretary, Alliance
        "Tim Kollars",    // Board President, Alliance
        "Tim Devlin",     // Trainer, Alliance
        "Stephanie Brandyberry", // Principal, Alma
        "Hannah Sindelar", // AD Secretary, Alma
        "Nick Simonson",  // Board President, Alma
        "Brittney Biskup", // Guidance Counselor, Alma
    ];

    let mut produced: Vec<String> = Vec::new();
    for entry in &schools {
        let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        produced.extend(
            nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)
                .expect("the nsaa coach rows parse")
                .into_iter()
                .map(|coach| coach.name),
        );
    }
    for person in office_people {
        assert!(
            !produced.contains(&person.to_string()),
            "{person} works in the office, not on the track"
        );
    }

    // The labels really are published — the parser sees and rejects them.
    let labels: Vec<&str> = schools
        .iter()
        .flat_map(|school| school.roles.iter())
        .map(|role| role.label.as_str())
        .collect();
    for office in [
        "Superintendent",
        "Principal",
        "AD Secretary",
        "Trainer",
        "Board President",
        "Guidance Counselor",
        "Student Council Sponsor",
    ] {
        assert!(
            labels.contains(&office),
            "{office} is a published row label"
        );
        assert!(
            parse_nsaa_row(office).is_none(),
            "{office} is not a coach/AD row"
        );
    }
    assert!(parse_nsaa_row("Assistant Athletic Director").is_some());
    assert_eq!(parse_nsaa_row("Athletic Director Secretary"), None);
}

#[test]
fn nsaa_office_row_is_ignored_but_the_same_persons_coaching_row_is_kept() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");

    // Alliance publishes Nate Lanik as Guidance Counselor *and* as both track coaches: the office
    // row contributes nothing, the sport rows contribute exactly two entities.
    let alliance = schools
        .iter()
        .find(|school| school.name == "Alliance")
        .expect("Alliance in the fixture");
    assert!(alliance
        .roles
        .iter()
        .any(|role| role.label == "Guidance Counselor" && role.name == "Nate Lanik"));
    let (_, alliance_id) = parse_nsaa_school(alliance, &url_of(alliance), OBSERVED_ON);
    let alliance_coaches = nsaa_coaches(alliance, &alliance_id, &url_of(alliance), OBSERVED_ON)
        .expect("the nsaa coach rows parse");
    assert_eq!(alliance_coaches.len(), 5, "Alliance: 1 AD + 2 XC + 2 track");
    let lanik: Vec<&CanonicalCoach> = alliance_coaches
        .iter()
        .filter(|coach| coach.name == "Nate Lanik")
        .collect();
    assert_eq!(lanik.len(), 2);
    for coach in lanik {
        assert_eq!(coach.role, CoachRole::HeadCoach);
        assert_eq!(coach.sport, Some(Sport::OutdoorTrack));
    }
    // `Unified Track & Field` also lists Nate Lanik and is not a census sport.
    assert!(alliance
        .roles
        .iter()
        .any(|role| role.label == "Unified Track & Field"));

    // Anselmo-Merna publishes Chanc McIntosh as Principal and as Activities/Athletic Director:
    // one AD entity, and no second entity from the Principal row.
    let anselmo = schools
        .iter()
        .find(|school| school.name == "Anselmo-Merna")
        .expect("Anselmo-Merna in the fixture");
    let (_, anselmo_id) = parse_nsaa_school(anselmo, &url_of(anselmo), OBSERVED_ON);
    let anselmo_coaches = nsaa_coaches(anselmo, &anselmo_id, &url_of(anselmo), OBSERVED_ON)
        .expect("the nsaa coach rows parse");
    assert_eq!(anselmo_coaches.len(), 3, "1 AD + 2 track");
    assert_eq!(
        anselmo_coaches
            .iter()
            .filter(|coach| coach.name == "Chanc McIntosh")
            .count(),
        1
    );

    // Ansley publishes Garrod Fernau as Principal *and* as Assistant Athletic Director: he is
    // present as a director, because a real director row names him.
    let ansley = schools
        .iter()
        .find(|school| school.name == "Ansley")
        .expect("Ansley in the fixture");
    assert!(ansley
        .roles
        .iter()
        .any(|role| role.label == "Principal" && role.name == "Garrod Fernau"));
    let (_, ansley_id) = parse_nsaa_school(ansley, &url_of(ansley), OBSERVED_ON);
    let ansley_coaches = nsaa_coaches(ansley, &ansley_id, &url_of(ansley), OBSERVED_ON)
        .expect("the nsaa coach rows parse");
    assert_eq!(ansley_coaches.len(), 6, "2 AD + 2 XC + 2 track");
    assert_eq!(
        ansley_coaches
            .iter()
            .filter(|coach| coach.name == "Garrod Fernau")
            .count(),
        1,
        "his Assistant Athletic Director row imports him; the Principal row does not"
    );
}

#[test]
fn nsaa_fixture_yields_exactly_the_verified_entities() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let mut by_school: Vec<(String, Vec<String>)> = Vec::new();
    for entry in &schools {
        let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        let mut rows: Vec<String> = nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)
            .expect("the nsaa coach rows parse")
            .into_iter()
            .map(|coach| {
                format!(
                    "{}|{:?}|{:?}|{:?}",
                    coach.name, coach.sport, coach.gender, coach.role
                )
            })
            .collect();
        rows.sort();
        by_school.push((entry.name.clone(), rows));
    }

    let adams = &by_school[0];
    assert_eq!(adams.0, "Adams Central");
    assert_eq!(
        adams.1,
        vec![
            "Alan Frank|None|Mixed|AthleticDirector",
            "Aub Boucher|None|Mixed|AthleticDirector",
            "Toni Fowler|Some(CrossCountry)|Boys|HeadCoach",
            "Toni Fowler|Some(CrossCountry)|Girls|HeadCoach",
            "Toni Fowler|Some(OutdoorTrack)|Girls|HeadCoach",
            "Zeb Noyd|Some(OutdoorTrack)|Boys|HeadCoach",
        ]
    );
    let ansley = by_school
        .iter()
        .find(|(name, _)| name == "Ansley")
        .expect("Ansley");
    assert_eq!(
        ansley.1,
        vec![
            "Aaron Wagner|None|Mixed|AthleticDirector",
            "Cayley Bailey|Some(CrossCountry)|Boys|HeadCoach",
            "Cayley Bailey|Some(CrossCountry)|Girls|HeadCoach",
            "Garrod Fernau|None|Mixed|AthleticDirector",
            "Jamee Smith|Some(OutdoorTrack)|Boys|HeadCoach",
            "Jamee Smith|Some(OutdoorTrack)|Girls|HeadCoach",
        ]
    );
    let total: usize = by_school.iter().map(|(_, rows)| rows.len()).sum();
    assert_eq!(total, 42);
}

#[test]
fn nsaa_coop_annotations_are_stripped_from_names() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let ansley = schools
        .iter()
        .find(|school| school.name == "Ansley")
        .expect("Ansley in the fixture");
    assert!(
        ansley
            .roles
            .iter()
            .any(|role| role.name == "Cayley Bailey (Co-op w/Litchfield)"),
        "the raw cell text is kept on the parsed row"
    );

    let (_, school_id) = parse_nsaa_school(ansley, &url_of(ansley), OBSERVED_ON);
    let names: Vec<String> = nsaa_coaches(ansley, &school_id, &url_of(ansley), OBSERVED_ON)
        .expect("the nsaa coach rows parse")
        .into_iter()
        .map(|coach| coach.name)
        .collect();
    assert!(names.contains(&"Cayley Bailey".to_string()));
    assert!(names.contains(&"Jamee Smith".to_string()));
    assert!(!names.iter().any(|name| name.contains("Co-op")));

    // The other shapes the source publishes, verbatim from the 2026-09-20 full capture.
    assert_eq!(
        split_person_names("Cayley Bailey (Co-op w/Litchfield)").expect("the name cell parses"),
        vec!["Cayley Bailey"]
    );
    assert_eq!(
        split_person_names("Derek Mahony Co-op w/Wheeler Central").expect("the name cell parses"),
        vec!["Derek Mahony"]
    );
    assert_eq!(
        split_person_names("Jenna Landgren Co-op w/Wheeler Central").expect("the name cell parses"),
        vec!["Jenna Landgren"]
    );
    assert_eq!(
        split_person_names("Carrie Ourada (Co-oop w/ Loup County").expect("the name cell parses"),
        vec!["Carrie Ourada"]
    );
    assert!(split_person_names("Co-op w/Loup CIty")
        .expect("the name cell parses")
        .is_empty());
    assert!(split_person_names("   ")
        .expect("the name cell parses")
        .is_empty());
    assert_eq!(
        split_person_names("Betsy Rall & Amy Sokol").expect("the name cell parses"),
        vec!["Betsy Rall", "Amy Sokol"]
    );
    assert_eq!(
        split_person_names("Jeff Tescher, Jeff Tescher").expect("the name cell parses"),
        vec!["Jeff Tescher"]
    );
    assert_eq!(
        split_person_names("Dr. Dan Schinzel").expect("the name cell parses"),
        vec!["Dan Schinzel"]
    );
}

#[test]
fn nsaa_fixture_reports_its_own_fill_rate_and_entity_count() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    let mut slots = 0usize;
    let mut named = 0usize;
    let mut coaches = 0usize;
    for entry in &schools {
        let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        coaches += nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)
            .expect("the nsaa coach rows parse")
            .len();
        for role in &entry.roles {
            if matches!(
                parse_nsaa_row(&role.label),
                Some(NsaaRow::SportCoach { .. })
            ) {
                slots += 1;
                if !split_person_names(&role.name)
                    .expect("the name cell parses")
                    .is_empty()
                {
                    named += 1;
                }
            }
        }
    }
    assert_eq!(
        slots, 30,
        "8 schools × 4 TF/XC sides, minus Anselmo-Merna's two XC sides (it sponsors neither)"
    );
    assert_eq!(
        named, 30,
        "every published TF/XC coach slot is named: 30/30"
    );
    assert_eq!(
        coaches, 42,
        "unique coach entities across the 8 fixture schools"
    );
}

#[test]
fn nsaa_entities_carry_no_emails_anywhere() {
    let schools = parse_nsaa_directory(NSAA_PAGE).expect("the nsaa directory parses");
    for entry in &schools {
        let (school, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        assert!(
            !serialized(&school).contains('@'),
            "no email in {}",
            school.name
        );
        for coach in nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)
            .expect("the nsaa coach rows parse")
        {
            assert_eq!(coach.professional_email, None);
            assert_eq!(coach.phone, None);
            assert!(
                !serialized(&coach).contains('@'),
                "no email or handle in coach {}",
                coach.name
            );
            assert_eq!(coach.evidence.len(), 1);
            assert_eq!(coach.evidence[0].observed_on, OBSERVED_ON);
            assert_eq!(coach.evidence[0].source.id, "nsaa");
        }
    }
}

#[test]
fn nd_entities_carry_no_emails_anywhere() {
    let (school, school_id) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)
        .expect("the ndhsaa page parses")
        .expect("school");
    assert!(!serialized(&school).contains('@'));
    let mut produced = nd_ad_coaches(
        &parse_nd_staff(ND_PAGE).expect("the ndhsaa staff lines parse"),
        &school_id,
        "u",
        OBSERVED_ON,
    );
    produced.extend(nd_sport_coaches(
        &parse_nd_offerings(ND_PAGE).expect("the ndhsaa offering rows parse"),
        &school_id,
        "u",
        OBSERVED_ON,
    ));
    assert!(!produced.is_empty());
    for coach in &produced {
        assert_eq!(coach.professional_email, None);
        assert_eq!(coach.phone, None);
        assert!(
            !serialized(coach).contains('@'),
            "no email in coach {}",
            coach.name
        );
    }
    // The fixture page itself has no email at all — the provider publishes no email layer.
    assert!(!email_regex()
        .expect("the email probe regex compiles")
        .is_match(ND_PAGE));
}

#[test]
fn nsaa_malformed_input_yields_no_rows() {
    assert!(parse_nsaa_directory("<!doctype html><html>nothing</html>")
        .expect("the nsaa directory parses")
        .is_empty());
    assert!(parse_nsaa_directory("")
        .expect("the nsaa directory parses")
        .is_empty());
    // A heading without a closing tag, and a heading with an empty name, are both skipped.
    assert!(parse_nsaa_directory(r#"<h1 class="mt-3">Broken"#)
        .expect("the nsaa directory parses")
        .is_empty());
    assert!(
        parse_nsaa_directory(r#"<h1 class="mt-3">   </h1><table></table>"#)
            .expect("the nsaa directory parses")
            .is_empty()
    );
    assert_eq!(parse_nsaa_row(""), None);
    let school = NsaaSchool {
        name: String::new(),
        city: None,
        enrollment: None,
        homepage: None,
        roles: Vec::new(),
    };
    let (_, school_id) = parse_nsaa_school(&school, &url_of(&school), OBSERVED_ON);
    assert!(
        nsaa_coaches(&school, &school_id, &url_of(&school), OBSERVED_ON)
            .expect("the nsaa coach rows parse")
            .is_empty()
    );
}

#[tokio::test]
async fn collect_skips_providers_it_was_not_asked_for() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = census_store::Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
    };

    let options = Options {
        states: vec![UsJurisdiction::Iowa],
        observed_on: OBSERVED_ON.to_string(),
        ..Options::default()
    };
    let report = collect(&ctx, &options)
        .await
        .expect("collect returns a report");
    assert_eq!(report.rows, 0);
    assert_eq!(report.with_email, 0);
    assert_eq!(
        report.requests, 0,
        "no provider ran, so no request was sent"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("no provider selected")),
        "the report says which states were asked for: {:?}",
        report.notes
    );
}
