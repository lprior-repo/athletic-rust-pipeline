use super::*;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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

fn url_of(school: &NsaaSchool) -> String {
    nsaa_school_url(&school.name)
}

fn serialized<T: serde::Serialize>(value: &T) -> TestResult<String> {
    Ok(serde_json::to_string(value)?)
}

#[test]
fn nd_index_lists_every_member_school() -> TestResult {
    let members = parse_nd_school_refs(ND_INDEX)?;
    check!(eq;
        members.len(),
        169,
        "the index carries all 169 member schools"
    );

    let ids: HashSet<&str> = members.iter().map(|member| member.id.as_str()).collect();
    check!(eq; ids.len(), members.len(), "ids are unique after dedupe");
    check!(ids.contains("1045"));

    let sheyenne = members
        .iter()
        .find(|member| member.id == "1045")
        .ok_or("West Fargo Sheyenne is a member")?;
    check!(eq; sheyenne.slug, "west-fargo-sheyenne");
    check!(eq;
        sheyenne.url(),
        "https://ndhsaa.com/schools/1045/west-fargo-sheyenne"
    );
    Ok(())
}

#[test]
fn nd_index_dedupes_and_ignores_other_links() -> TestResult {
    let html = r#"
            <a href="https://ndhsaa.com/schools/7/bismarck">Bismarck</a>
            <a href="/schools/7/bismarck-high">Bismarck again</a>
            <a href="/schools">All schools</a>
            <a href="https://ndhsaa.com/athletics/track-boys">Track</a>
            <a href="/schools/92/alexander">Alexander</a>
        "#;
    let members = parse_nd_school_refs(html)?;
    check!(eq; members.len(), 2);
    check!(eq; members[0].slug, "bismarck", "first link per id wins");
    check!(eq; members[1].id, "92");
    Ok(())
}

#[test]
fn nd_school_page_parses_school_metadata() -> TestResult {
    let (school, school_id) =
        parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)?.ok_or("fixture has a heading")?;

    check!(eq; school.name, "West Fargo Sheyenne High School");
    check!(eq;
        school.normalized_name, "west fargo sheyenne",
        "normalize_name drops the `High School` suffix"
    );
    check!(eq; school.state, Some(UsJurisdiction::NorthDakota));
    check!(eq; school.association.as_deref(), Some("ndhsaa"));
    check!(eq; school.city.as_deref(), Some("West Fargo"));
    check!(eq;
        school.enrollment,
        Some(1399),
        "1,399 students enrolled in 2025"
    );
    check!(eq;
        school.school_website.as_deref(),
        Some("https://www.west-fargo.k12.nd.us/shs/activities")
    );
    check!(eq; school.source_identities.len(), 1);
    check!(eq;
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "ndhsaa".to_string()
        }
    );
    check!(eq; school.source_identities[0].id, "1045");
    check!(eq;
        school.source_identities[0].url.as_deref(),
        Some("https://ndhsaa.com/schools/1045/west-fargo-sheyenne")
    );
    check!(eq; school.evidence.len(), 1);
    check!(eq; school.evidence[0].observed_on, OBSERVED_ON);
    check!(eq; school.evidence[0].source.id, "ndhsaa");
    check!(eq;
        school.evidence[0].source.url.as_deref(),
        Some("https://ndhsaa.com/schools/1045/west-fargo-sheyenne")
    );
    check!(eq;
        school_id,
        CanonicalSchool::mint(
            UsJurisdiction::NorthDakota,
            &school.name,
            &school.normalized_name,
            Some("West Fargo"),
        )
    );
    let (_, same_school) = CanonicalSchool::new(
        UsJurisdiction::NorthDakota,
        "West Fargo Sheyenne HS",
        normalize_name("West Fargo Sheyenne HS"),
        Some("West Fargo"),
    );
    check!(eq; school_id, same_school);
    Ok(())
}

#[test]
fn nd_school_page_never_stores_phone_fax_or_address() -> TestResult {
    let (school, _) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)?.ok_or("school")?;
    let json = serialized(&school)?;
    check!(
        !json.contains("356-2160"),
        "the school phone number is not stored"
    );
    check!(!json.contains("499-6687"), "the fax number is not stored");
    check!(
        !json.contains("800 40th Ave E."),
        "the street address is not stored"
    );
    Ok(())
}

#[test]
fn nd_ad_coaches_include_ad_and_activities_director_only() -> TestResult {
    let (_, school_id) =
        parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)?.ok_or("school")?;
    let staff = parse_nd_staff(ND_PAGE)?;
    let coaches = nd_ad_coaches(
        &staff,
        &school_id,
        "https://ndhsaa.com/schools/1045/west-fargo-sheyenne",
        OBSERVED_ON,
    );

    let names: Vec<&str> = coaches.iter().map(|coach| coach.name.as_str()).collect();
    check!(eq;
        names,
        vec!["Logan Midthun", "James Moe", "Corissa Kolesar"],
        "Athletic Director and Activities Director rows collapse to one entity per person"
    );
    for coach in &coaches {
        check!(eq; coach.role, CoachRole::AthleticDirector);
        check!(eq; coach.sport, None, "directors are school-wide roles");
        check!(eq; coach.gender, Gender::Mixed);
        check!(eq; coach.professional_email, None);
        check!(eq; coach.phone, None);
        check!(eq; coach.evidence.len(), 1);
        check!(eq; coach.evidence[0].observed_on, OBSERVED_ON);
    }

    let labels: Vec<&str> = staff.iter().map(|role| role.label.as_str()).collect();
    for office in [
        "Superintendent",
        "Assistant Superintendent",
        "Principal",
        "Vice/Assistant Principal",
        "Business Manager",
        "Tech Director",
    ] {
        check!(
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
        check!(
            !names.contains(&person),
            "{person} ({label}) must never be emitted as a coach or director"
        );
        check!(
            parse_nd_role(label).is_none(),
            "{label} is not a director role"
        );
    }
    Ok(())
}

#[test]
fn nd_page_without_ad_yields_no_director_rows() -> TestResult {
    let (school, school_id) =
        parse_nd_school_page(ND_PAGE_NO_AD, &mandan_classical(), OBSERVED_ON)?
            .ok_or("the page still yields a school")?;
    check!(eq; school.name, "Mandan Classical Academy");
    check!(eq; school.city.as_deref(), Some("Mandan"));
    check!(eq; school.association.as_deref(), Some("ndhsaa"));

    let staff = parse_nd_staff(ND_PAGE_NO_AD)?;
    check!(
        !staff
            .iter()
            .any(|role| role.label.to_ascii_lowercase().contains("director")),
        "fixture really carries no director line"
    );
    check!(
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
    check!(
        coaches.is_empty(),
        "a superintendent is not an athletic director, however senior"
    );
    Ok(())
}

#[test]
fn nd_offering_rows_and_coop_annotations_parse() -> TestResult {
    let offerings = parse_nd_offerings(ND_PAGE)?;
    check!(eq; offerings.len(), 28, "one row per published offering");

    let cross_country = offerings
        .iter()
        .find(|offering| offering.label == "Boys' Cross Country")
        .ok_or("Boys' Cross Country is offered")?;
    check!(eq; cross_country.coaches, vec!["Troy Thorson", "Jared Slinde"]);
    check!(eq; cross_country.co_op, None);

    let hockey = offerings
        .iter()
        .find(|offering| offering.label == "Boys' Ice Hockey")
        .ok_or("Boys' Ice Hockey is offered")?;
    check!(eq;
        hockey.co_op.as_deref(),
        Some("West Fargo Sheyenne"),
        "the co-op annotation is recorded, not left inside the sport label"
    );

    let blank = offerings
        .iter()
        .find(|offering| offering.label == "Debate")
        .ok_or("Debate is offered")?;
    check!(
        blank.coaches.is_empty(),
        "a blank coach cell yields no names"
    );
    Ok(())
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
fn nd_sport_coaches_are_names_only() -> TestResult {
    let (_, school_id) =
        parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)?.ok_or("school")?;
    let offerings = parse_nd_offerings(ND_PAGE)?;
    let coaches = nd_sport_coaches(
        &offerings,
        &school_id,
        "https://ndhsaa.com/schools/1045/west-fargo-sheyenne",
        OBSERVED_ON,
    );

    check!(eq; coaches.len(), 6);
    for coach in &coaches {
        check!(eq;
            coach.role,
            CoachRole::Unknown,
            "NDHSAA publishes no head/assistant split"
        );
        check!(coach.sport.is_some());
        check!(eq; coach.professional_email, None);
        check!(eq; coach.phone, None);
    }
    let girls_track = coaches
        .iter()
        .find(|coach| coach.sport == Some(Sport::OutdoorTrack) && coach.gender == Gender::Girls)
        .ok_or("girls track coach")?;
    check!(eq; girls_track.name, "Jaime Watson");
    check!(
        coaches
            .iter()
            .all(|coach| { !coach.evidence.is_empty() && coach.evidence[0].source.id == "ndhsaa" }),
        "every coach cites the page it came from"
    );
    check!(!coaches.iter().any(|coach| coach.name == "Tim Brandt"));
    Ok(())
}

#[test]
fn nd_fixture_reports_its_own_fill_rate() -> TestResult {
    let offerings = parse_nd_offerings(ND_PAGE)?;
    let slots: Vec<&NdOffering> = offerings
        .iter()
        .filter(|offering| parse_nd_sport(&offering.label).is_some())
        .collect();
    let named = slots
        .iter()
        .filter(|offering| !offering.coaches.is_empty())
        .count();
    check!(eq;
        slots.len(),
        4,
        "the fixture page offers boys/girls XC and track"
    );
    check!(eq;
        named, 4,
        "all four TF/XC coach slots are named on this page"
    );
    Ok(())
}

#[test]
fn nd_malformed_input_yields_no_rows() -> TestResult {
    check!(parse_nd_school_refs("<html>no links here</html>")?.is_empty());
    check!(parse_nd_school_refs("")?.is_empty());
    check!(parse_nd_school_page(
        "<html><body>Nothing</body></html>",
        &sheyenne(),
        OBSERVED_ON
    )?
    .is_none());
    check!(parse_nd_staff("not html at all")?.is_empty());
    check!(parse_nd_offerings("<table><tr><td>only one cell</td></tr></table>")?.is_empty());
    check!(parse_nd_school_page(
        "<h1>   </h1><p>Address: x, Fargo, ND 58102</p>",
        &sheyenne(),
        OBSERVED_ON
    )?
    .is_none());
    Ok(())
}

#[test]
fn nsaa_form_option_list_yields_the_member_schools() -> TestResult {
    let names = parse_nsaa_school_names(NSAA_FORM)?;
    check!(eq; names.len(), 312, "the form lists every member school");
    check!(eq; names[0], "Adams Central");
    check!(eq; names[1], "Ainsworth");
    check!(eq; names[311], "Yutan");
    check!(
        !names.iter().any(|name| name == NSAA_ALL_SCHOOLS),
        "the bulk sentinel is not a school"
    );
    check!(
        !names.iter().any(|name| name.contains("Select a school")),
        "the disabled placeholder is not a school"
    );
    check!(eq;
        names
            .iter()
            .filter(|name| name.as_str() == "Adams Central")
            .count(),
        1,
        "names are unique"
    );

    let bulk: Vec<String> = parse_nsaa_directory(NSAA_PAGE)?
        .into_iter()
        .map(|school| school.name)
        .collect();
    for name in &bulk {
        check!(
            names.contains(name),
            "{name} is an option and a block heading"
        );
    }

    check!(parse_nsaa_school_names("")?.is_empty());
    check!(parse_nsaa_school_names("<select></select>")?.is_empty());
    check!(parse_nsaa_school_names("<option></option>")?.is_empty());
    check!(
        parse_nsaa_school_names("<option disabled>Select a school to view...</option>")?.is_empty()
    );
    Ok(())
}

#[test]
fn nsaa_school_url_round_trips_the_published_name() -> TestResult {
    check!(eq;
        nsaa_school_url("Adams Central"),
        "https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Adams+Central",
        "the `+` form is what `byte_serialize` emits; the server decoded it to the same 15,579-byte \
         page as the `%20` form (live 2026-09-20T14:39:13Z, HTTP 200, byte-identical)"
    );
    check!(eq;
        nsaa_school_url("Anselmo-Merna"),
        "https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Anselmo-Merna",
        "hyphens are safe and stay literal"
    );

    for name in parse_nsaa_school_names(NSAA_FORM)? {
        let url = url::Url::parse(&nsaa_school_url(&name))?;
        let decoded = url
            .query_pairs()
            .find(|(key, _)| key == "school")
            .map(|(_, value)| value.into_owned())
            .ok_or("school query parameter")?;
        check!(eq; decoded, name, "round-trip for {name}");
    }
    Ok(())
}

#[test]
fn nsaa_single_school_page_matches_the_bulk_block() -> TestResult {
    let single = parse_nsaa_directory(NSAA_SCHOOL_GET)?;
    check!(eq; single.len(), 1, "one school per single-school response");
    let entry = &single[0];
    check!(eq; entry.name, "Adams Central");
    check!(eq; entry.roles.len(), 34);
    check!(eq; entry.city.as_deref(), Some("Hastings"));
    check!(eq; entry.enrollment, Some(215));
    check!(eq;
        entry.homepage.as_deref(),
        Some("http://www.adamscentral.us/")
    );

    let bulk = parse_nsaa_directory(NSAA_PAGE)?;
    let bulk_adams = bulk
        .iter()
        .find(|school| school.name == "Adams Central")
        .ok_or("Adams Central in the bulk slice")?;
    check!(eq; entry.roles, bulk_adams.roles);

    let url = nsaa_school_url("Adams Central");
    let (school, school_id) = parse_nsaa_school(entry, &url, OBSERVED_ON);
    let coaches = nsaa_coaches(entry, &school_id, &url, OBSERVED_ON)?;
    let (_, bulk_id) = parse_nsaa_school(bulk_adams, &url, OBSERVED_ON);
    check!(eq; school_id, bulk_id, "one canonical id either way");
    check!(eq; coaches.len(), 6);
    check!(eq;
        school.evidence[0].source.url.as_deref(),
        Some(nsaa_school_url("Adams Central").as_str()),
        "evidence cites the request URL that produced the row"
    );
    Ok(())
}

#[test]
fn nsaa_directory_parses_every_school_block() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let names: Vec<&str> = schools.iter().map(|school| school.name.as_str()).collect();
    check!(eq;
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
    check!(eq; adams.city.as_deref(), Some("Hastings"));
    check!(eq; adams.enrollment, Some(215));
    check!(eq;
        adams.homepage.as_deref(),
        Some("http://www.adamscentral.us/")
    );
    check!(eq;
        adams.roles.len(),
        34,
        "one row per published staff/coach line"
    );

    let coop_rows = schools
        .iter()
        .flat_map(|school| school.roles.iter())
        .filter(|role| role.co_op)
        .count();
    check!(eq;
        coop_rows, 32,
        "rows the directory highlights as co-op-shared carry `class='table-info'`"
    );
    check!(eq;
        adams.roles.iter().filter(|role| role.co_op).count(),
        3,
        "Adams Central's co-op rows: Softball and the two Swimming placeholders"
    );
    let allen = schools
        .iter()
        .find(|school| school.name == "Allen")
        .ok_or("Allen in the fixture")?;
    check!(eq; allen.roles.iter().filter(|role| role.co_op).count(), 13);
    let ansley = schools
        .iter()
        .find(|school| school.name == "Ansley")
        .ok_or("Ansley in the fixture")?;
    check!(eq; ansley.roles.iter().filter(|role| role.co_op).count(), 10);
    Ok(())
}

#[test]
fn nsaa_sport_rows_map_to_head_coach_sport_and_gender() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let adams = &schools[0];
    check!(eq;
        parse_nsaa_row("Cross-Country (Boys)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::CrossCountry,
            gender: Gender::Boys
        })
    );
    check!(eq;
        parse_nsaa_row("Cross-Country (Girls)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::CrossCountry,
            gender: Gender::Girls
        })
    );
    check!(eq;
        parse_nsaa_row("Track & Field (Boys)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys
        })
    );
    check!(eq;
        parse_nsaa_row("Track & Field (Girls)"),
        Some(NsaaRow::SportCoach {
            sport: Sport::OutdoorTrack,
            gender: Gender::Girls
        })
    );
    check!(eq;
        parse_nsaa_row("Unified Track & Field"),
        None,
        "a distinct NSAA activity"
    );
    check!(eq; parse_nsaa_row("Strength Coach"), None);
    check!(eq; parse_nsaa_row("Volleyball"), None);

    let (_, school_id) = parse_nsaa_school(adams, &url_of(adams), OBSERVED_ON);
    let coaches = nsaa_coaches(adams, &school_id, &url_of(adams), OBSERVED_ON)?;
    let track_boys = coaches
        .iter()
        .find(|coach| {
            coach.sport == Some(Sport::OutdoorTrack)
                && coach.gender == Gender::Boys
                && coach.name != "Toni Fowler"
        })
        .ok_or("boys track coach")?;
    check!(eq; track_boys.name, "Zeb Noyd");
    check!(eq; track_boys.role, CoachRole::HeadCoach);
    let xc_boys = coaches
        .iter()
        .find(|coach| coach.sport == Some(Sport::CrossCountry) && coach.gender == Gender::Boys)
        .ok_or("boys XC coach")?;
    check!(eq; xc_boys.name, "Toni Fowler");
    check!(eq; xc_boys.role, CoachRole::HeadCoach);
    Ok(())
}

#[test]
fn nsaa_multi_name_cells_split_into_one_entity_per_person() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let ainsworth = schools
        .iter()
        .find(|school| school.name == "Ainsworth")
        .ok_or("Ainsworth in the fixture")?;
    let (_, school_id) = parse_nsaa_school(ainsworth, &url_of(ainsworth), OBSERVED_ON);
    let coaches = nsaa_coaches(ainsworth, &school_id, &url_of(ainsworth), OBSERVED_ON)?;

    let xc: Vec<&str> = coaches
        .iter()
        .filter(|coach| coach.sport == Some(Sport::CrossCountry))
        .map(|coach| coach.name.as_str())
        .collect();
    check!(eq;
        xc,
        vec![
            "Trey Schlueter",
            "Katie Winters",
            "Trey Schlueter",
            "Katie Winters"
        ],
        "`Trey Schlueter/Katie Winters` is two people, not one name field"
    );
    check!(!xc.iter().any(|name| name.contains('/')));
    Ok(())
}

#[test]
fn nsaa_director_rows_map_to_athletic_director() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let adams = &schools[0];
    let (school, school_id) = parse_nsaa_school(adams, &url_of(adams), OBSERVED_ON);
    check!(eq; school.state, Some(UsJurisdiction::Nebraska));
    check!(eq; school.association.as_deref(), Some("nsaa"));
    check!(eq; school.city.as_deref(), Some("Hastings"));
    check!(eq; school.enrollment, Some(215));
    check!(eq; school.source_identities.len(), 1);
    check!(eq;
        school.source_identities[0].namespace,
        SourceNamespace::AssociationSchool {
            association: "nsaa".to_string()
        }
    );
    check!(eq;
        school.source_identities[0].id, "Adams Central",
        "NSAA publishes no numeric id, so the published name is the provider key"
    );

    let coaches = nsaa_coaches(adams, &school_id, &url_of(adams), OBSERVED_ON)?;
    let directors: Vec<&str> = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .map(|coach| coach.name.as_str())
        .collect();
    check!(eq;
        directors,
        vec!["Alan Frank", "Aub Boucher"],
        "Activities Director + Athletic Director collapse, the assistant director is an AD row"
    );
    for coach in coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
    {
        check!(eq; coach.sport, None);
        check!(eq; coach.gender, Gender::Mixed);
    }
    Ok(())
}

#[test]
fn nsaa_office_roles_are_never_emitted() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let office_people = [
        "Shawn Scott",
        "Scott Harrington",
        "Mattison Tinant",
        "Dave Johnson",
        "Becky Fisher",
        "Sean Vonderfecht",
        "Dale Hafer",
        "Kari Painter",
        "Brad Wilkins",
        "Jerry Bockman",
        "Mike Pattee",
        "Chris Blohm",
        "Becky Stapleton",
        "Jason Olesen",
        "Kim Jonas",
        "Chrissy Slingsby",
        "Roger Thomsen",
        "Carlene Abbott",
        "Bobbi Sorensen",
        "Aaron Klingelhoefer",
        "Lloyd McIntyre",
        "Molli Miller",
        "Dr. Troy Unzicker",
        "Marissa Rotness",
        "Tim Kollars",
        "Tim Devlin",
        "Stephanie Brandyberry",
        "Hannah Sindelar",
        "Nick Simonson",
        "Brittney Biskup",
    ];

    let mut produced: Vec<String> = Vec::new();
    for entry in &schools {
        let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        produced.extend(
            nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)?
                .into_iter()
                .map(|coach| coach.name),
        );
    }
    for person in office_people {
        check!(
            !produced.contains(&person.to_string()),
            "{person} works in the office, not on the track"
        );
    }

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
        check!(
            labels.contains(&office),
            "{office} is a published row label"
        );
        check!(
            parse_nsaa_row(office).is_none(),
            "{office} is not a coach/AD row"
        );
    }
    check!(parse_nsaa_row("Assistant Athletic Director").is_some());
    check!(eq; parse_nsaa_row("Athletic Director Secretary"), None);
    Ok(())
}

#[test]
fn nsaa_office_row_is_ignored_but_the_same_persons_coaching_row_is_kept() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;

    let alliance = schools
        .iter()
        .find(|school| school.name == "Alliance")
        .ok_or("Alliance in the fixture")?;
    check!(alliance
        .roles
        .iter()
        .any(|role| role.label == "Guidance Counselor" && role.name == "Nate Lanik"));
    let (_, alliance_id) = parse_nsaa_school(alliance, &url_of(alliance), OBSERVED_ON);
    let alliance_coaches = nsaa_coaches(alliance, &alliance_id, &url_of(alliance), OBSERVED_ON)?;
    check!(eq; alliance_coaches.len(), 5, "Alliance: 1 AD + 2 XC + 2 track");
    let lanik: Vec<&CanonicalCoach> = alliance_coaches
        .iter()
        .filter(|coach| coach.name == "Nate Lanik")
        .collect();
    check!(eq; lanik.len(), 2);
    for coach in lanik {
        check!(eq; coach.role, CoachRole::HeadCoach);
        check!(eq; coach.sport, Some(Sport::OutdoorTrack));
    }
    check!(alliance
        .roles
        .iter()
        .any(|role| role.label == "Unified Track & Field"));

    let anselmo = schools
        .iter()
        .find(|school| school.name == "Anselmo-Merna")
        .ok_or("Anselmo-Merna in the fixture")?;
    let (_, anselmo_id) = parse_nsaa_school(anselmo, &url_of(anselmo), OBSERVED_ON);
    let anselmo_coaches = nsaa_coaches(anselmo, &anselmo_id, &url_of(anselmo), OBSERVED_ON)?;
    check!(eq; anselmo_coaches.len(), 3, "1 AD + 2 track");
    check!(eq;
        anselmo_coaches
            .iter()
            .filter(|coach| coach.name == "Chanc McIntosh")
            .count(),
        1
    );

    let ansley = schools
        .iter()
        .find(|school| school.name == "Ansley")
        .ok_or("Ansley in the fixture")?;
    check!(ansley
        .roles
        .iter()
        .any(|role| role.label == "Principal" && role.name == "Garrod Fernau"));
    let (_, ansley_id) = parse_nsaa_school(ansley, &url_of(ansley), OBSERVED_ON);
    let ansley_coaches = nsaa_coaches(ansley, &ansley_id, &url_of(ansley), OBSERVED_ON)?;
    check!(eq; ansley_coaches.len(), 6, "2 AD + 2 XC + 2 track");
    check!(eq;
        ansley_coaches
            .iter()
            .filter(|coach| coach.name == "Garrod Fernau")
            .count(),
        1,
        "his Assistant Athletic Director row imports him; the Principal row does not"
    );
    Ok(())
}

#[test]
fn nsaa_fixture_yields_exactly_the_verified_entities() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let mut by_school: Vec<(String, Vec<String>)> = Vec::new();
    for entry in &schools {
        let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        let mut rows: Vec<String> = nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)?
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
    check!(eq; adams.0, "Adams Central");
    check!(eq;
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
        .ok_or("Ansley")?;
    check!(eq;
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
    check!(eq; total, 42);
    Ok(())
}

#[test]
fn nsaa_coop_annotations_are_stripped_from_names() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let ansley = schools
        .iter()
        .find(|school| school.name == "Ansley")
        .ok_or("Ansley in the fixture")?;
    check!(
        ansley
            .roles
            .iter()
            .any(|role| role.name == "Cayley Bailey (Co-op w/Litchfield)"),
        "the raw cell text is kept on the parsed row"
    );

    let (_, school_id) = parse_nsaa_school(ansley, &url_of(ansley), OBSERVED_ON);
    let names: Vec<String> = nsaa_coaches(ansley, &school_id, &url_of(ansley), OBSERVED_ON)?
        .into_iter()
        .map(|coach| coach.name)
        .collect();
    check!(names.contains(&"Cayley Bailey".to_string()));
    check!(names.contains(&"Jamee Smith".to_string()));
    check!(!names.iter().any(|name| name.contains("Co-op")));

    check!(eq;
        split_person_names("Cayley Bailey (Co-op w/Litchfield)")?,
        vec!["Cayley Bailey"]
    );
    check!(eq;
        split_person_names("Derek Mahony Co-op w/Wheeler Central")?,
        vec!["Derek Mahony"]
    );
    check!(eq;
        split_person_names("Jenna Landgren Co-op w/Wheeler Central")?,
        vec!["Jenna Landgren"]
    );
    check!(eq;
        split_person_names("Carrie Ourada (Co-oop w/ Loup County")?,
        vec!["Carrie Ourada"]
    );
    check!(split_person_names("Co-op w/Loup CIty")?.is_empty());
    check!(split_person_names("   ")?.is_empty());
    check!(eq;
        split_person_names("Betsy Rall & Amy Sokol")?,
        vec!["Betsy Rall", "Amy Sokol"]
    );
    check!(eq;
        split_person_names("Jeff Tescher, Jeff Tescher")?,
        vec!["Jeff Tescher"]
    );
    check!(eq;
        split_person_names("Dr. Dan Schinzel")?,
        vec!["Dan Schinzel"]
    );
    Ok(())
}

#[test]
fn nsaa_fixture_reports_its_own_fill_rate_and_entity_count() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    let mut slots = 0usize;
    let mut named = 0usize;
    let mut coaches = 0usize;
    for entry in &schools {
        let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        coaches += nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)?.len();
        for role in &entry.roles {
            if matches!(
                parse_nsaa_row(&role.label),
                Some(NsaaRow::SportCoach { .. })
            ) {
                slots += 1;
                if !split_person_names(&role.name)?.is_empty() {
                    named += 1;
                }
            }
        }
    }
    check!(eq;
        slots, 30,
        "8 schools × 4 TF/XC sides, minus Anselmo-Merna's two XC sides (it sponsors neither)"
    );
    check!(eq;
        named, 30,
        "every published TF/XC coach slot is named: 30/30"
    );
    check!(eq;
        coaches, 42,
        "unique coach entities across the 8 fixture schools"
    );
    Ok(())
}

#[test]
fn nsaa_entities_carry_no_emails_anywhere() -> TestResult {
    let schools = parse_nsaa_directory(NSAA_PAGE)?;
    for entry in &schools {
        let (school, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
        check!(
            !serialized(&school)?.contains('@'),
            "no email in {}",
            school.name
        );
        for coach in nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)? {
            check!(eq; coach.professional_email, None);
            check!(eq; coach.phone, None);
            check!(
                !serialized(&coach)?.contains('@'),
                "no email or handle in coach {}",
                coach.name
            );
            check!(eq; coach.evidence.len(), 1);
            check!(eq; coach.evidence[0].observed_on, OBSERVED_ON);
            check!(eq; coach.evidence[0].source.id, "nsaa");
        }
    }
    Ok(())
}

#[test]
fn nd_entities_carry_no_emails_anywhere() -> TestResult {
    let (school, school_id) =
        parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON)?.ok_or("school")?;
    check!(!serialized(&school)?.contains('@'));
    let mut produced = nd_ad_coaches(&parse_nd_staff(ND_PAGE)?, &school_id, "u", OBSERVED_ON);
    produced.extend(nd_sport_coaches(
        &parse_nd_offerings(ND_PAGE)?,
        &school_id,
        "u",
        OBSERVED_ON,
    ));
    check!(!produced.is_empty());
    for coach in &produced {
        check!(eq; coach.professional_email, None);
        check!(eq; coach.phone, None);
        check!(
            !serialized(coach)?.contains('@'),
            "no email in coach {}",
            coach.name
        );
    }
    check!(!email_regex()?.is_match(ND_PAGE));
    Ok(())
}

#[test]
fn nsaa_malformed_input_yields_no_rows() -> TestResult {
    check!(parse_nsaa_directory("<!doctype html><html>nothing</html>")?.is_empty());
    check!(parse_nsaa_directory("")?.is_empty());
    check!(parse_nsaa_directory(r#"<h1 class="mt-3">Broken"#)?.is_empty());
    check!(parse_nsaa_directory(r#"<h1 class="mt-3">   </h1><table></table>"#)?.is_empty());
    check!(eq; parse_nsaa_row(""), None);
    let school = NsaaSchool {
        name: String::new(),
        city: None,
        enrollment: None,
        homepage: None,
        roles: Vec::new(),
    };
    let (_, school_id) = parse_nsaa_school(&school, &url_of(&school), OBSERVED_ON);
    check!(nsaa_coaches(&school, &school_id, &url_of(&school), OBSERVED_ON)?.is_empty());
    Ok(())
}

#[test]
fn collect_skips_providers_it_was_not_asked_for() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = census_store::Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?;
            let ctx = AdapterContext {
                fetcher: &fetcher,
                store: &store,
                refresh: false,
                school_year: census_domain::model::SchoolYear::new(2026)
                    .ok_or("2026 is a season")?,
                observed_on: OBSERVED_ON.to_string(),
                recording: None,
            };

            let options = Options {
                states: vec![UsJurisdiction::Iowa],
                observed_on: OBSERVED_ON.to_string(),
                ..Options::default()
            };
            let report = collect(&ctx, &options).await?;
            check!(eq; report.rows, 0);
            check!(eq; report.with_email, 0);
            check!(eq;
                report.requests, 0,
                "no provider ran, so no request was sent"
            );
            check!(
                report
                    .notes
                    .iter()
                    .any(|note| note.contains("no provider selected")),
                "the report says which states were asked for: {:?}",
                report.notes
            );
            Ok(())
        })
}
