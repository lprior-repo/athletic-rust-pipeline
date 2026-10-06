mod summary_parity;
mod tenure;
use super::collect::requested_states;
use super::map::{absorb_summary, coach_entities, directory_school, team_sport};
use super::parse::{parse_directory, parse_summary};
use super::row::{coach_role, is_director};
use super::*;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport,
};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NC_DIRECTORY: &str =
    include_str!("../../tests/fixtures/coach_directories/nchsaa_directory_p1.json");
const GA_DIRECTORY: &str =
    include_str!("../../tests/fixtures/coach_directories/ghsa_directory_p2.json");
const NC_SUMMARY: &str =
    include_str!("../../tests/fixtures/coach_directories/nc_staff_summary_zcum49.json");
const IN_SUMMARY: &str =
    include_str!("../../tests/fixtures/coach_directories/in_staff_summary_qwugx2.json");
const ACCESS_DENIED: &str =
    include_str!("../../tests/fixtures/coach_directories/summary_orgid_200_accessdenied.xml");
const AL_MIDDLE_SCHOOL_SUMMARY: &str =
    include_str!("../../tests/fixtures/coach_directories/probe/AL/summary-SVXJDF.json");
const AK_DIRECTORY: &str =
    include_str!("../../tests/fixtures/coach_directories/probe/AK/directory-1.json");
const WY_DIRECTORY: &str =
    include_str!("../../tests/fixtures/coach_directories/probe/WY/directory-1.json");
const GOLDEN_DIRECTORY_ROWS: &str =
    include_str!("../../tests/fixtures/coach_directories/golden_directory_rows.json");

const OBSERVED_ON: &str = "2026-09-29T12:00:00Z";

fn current_contexts(file: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut contexts: std::collections::BTreeMap<String, Vec<String>> = serde_json::from_str(
        include_str!("../../tests/golden/coach_directories__census-contexts.json"),
    )?;
    Ok(contexts.remove(file).ok_or("a qualified summary fixture")?)
}

fn minted(state: UsJurisdiction, name: &str) -> (CanonicalSchool, census_domain::model::SchoolId) {
    CanonicalSchool::new(state, name, normalize_name(name))
}

fn admitted(
    state: UsJurisdiction,
    association: &str,
    row: &DirectorySchool,
    url: &str,
) -> Result<(CanonicalSchool, census_domain::model::SchoolId), Box<dyn std::error::Error>> {
    match directory_school(state, association, row, url, OBSERVED_ON)? {
        DirectoryAdmission::School(school, id) => Ok((*school, id)),
        other => Err(format!("directory row not admitted: {other:?}").into()),
    }
}

fn emitted(
    summary: &SchoolSummary,
    school_id: &census_domain::model::SchoolId,
    url: &str,
) -> Result<CoachEmission, Box<dyn std::error::Error>> {
    Ok(coach_entities(
        summary,
        school_id,
        url,
        OBSERVED_ON,
        census_domain::model::SchoolYear::new(2026).ok_or("valid test school year")?,
        &crate::net::cache::content_digest(NC_SUMMARY.as_bytes()),
    )?)
}

fn context_keys<'a>(coaches: impl IntoIterator<Item = &'a CanonicalCoach>) -> Vec<String> {
    let mut keys: Vec<String> = coaches
        .into_iter()
        .map(|coach| {
            format!(
                "{}|{}|{}",
                coach.name,
                coach.sport.map_or("none", Sport::stable_key),
                coach.gender.stable_key()
            )
        })
        .collect();
    keys.sort();
    keys
}

#[test]
fn a_directory_page_reads_every_association_row() -> TestResult {
    let page = parse_directory(NC_DIRECTORY.as_bytes())?;
    check!(eq; page.current_page, 1);
    check!(eq; page.total_pages, 1);
    check!(eq; page.total_results, 452);
    check!(eq; page.results.len(), 452);
    let row = page.results.first().ok_or("first school")?;
    check!(eq; row.name.as_deref(), Some("A.C. Reynolds High School"));
    check!(eq; row.short_code.as_deref(), Some("ZCUM49"));
    check!(eq; row.org_id.as_deref(), Some("6285bff87f770402d0000006"));
    check!(eq; row.city.as_deref(), Some("Asheville"));
    check!(eq; row.address.as_deref(), Some("1 Rocket Drive"));
    check!(eq;
        classification(&row.competition_levels).as_deref(),
        Some("6A")
    );
    Ok(())
}

#[test]
fn a_page_without_association_levels_still_reads() -> TestResult {
    let page = parse_directory(GA_DIRECTORY.as_bytes())?;
    check!(eq; page.current_page, 2);
    check!(eq; page.total_pages, 3);
    check!(eq; page.results.len(), 1000);
    let row = page.results.first().ok_or("first school")?;
    check!(eq; row.name.as_deref(), Some("Glynn Middle"));
    check!(eq; row.state_code.as_deref(), Some("GA"));
    check!(row.competition_levels.is_empty());
    check!(eq; classification(&row.competition_levels), None);
    Ok(())
}

#[test]
fn the_directory_row_mints_the_school_with_its_association_and_level() -> TestResult {
    let page = parse_directory(NC_DIRECTORY.as_bytes())?;
    let row = page.results.first().ok_or("first school")?;
    let (school, id) = admitted(
        UsJurisdiction::NorthCarolina,
        "NCHSAA",
        row,
        "https://example.test/states/NCHSAA/directory/1",
    )?;
    check!(eq; school.name, "A.C. Reynolds High School");
    check!(eq; school.city.as_deref(), Some("Asheville"));
    check!(eq; school.association.as_deref(), Some("NCHSAA"));
    check!(eq; school.classification.as_deref(), Some("6A"));
    check!(eq; school.state, Some(UsJurisdiction::NorthCarolina));
    check!(eq; school.source_identities.len(), 1);
    check!(eq; school.evidence.len(), 1);
    check!(eq;
        id.as_str(),
        CanonicalSchool::mint(
            UsJurisdiction::NorthCarolina,
            "A.C. Reynolds High School",
            &normalize_name("A.C. Reynolds High School")
        )
        .as_str()
    );
    Ok(())
}

#[test]
fn the_summary_maps_every_published_row() -> TestResult {
    let summary = parse_summary(NC_SUMMARY.as_bytes())?;
    check!(eq; summary.short_code, "ZCUM49");
    let (_, school_id) = minted(UsJurisdiction::NorthCarolina, "A.C. Reynolds High School");
    let coaches = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/ZCUM49/summary",
    )?
    .coaches;
    check!(eq;
        context_keys(coaches.iter()),
        current_contexts("nc_staff_summary_zcum49.json")?
    );
    let honea: Vec<&CanonicalCoach> = coaches
        .iter()
        .filter(|coach| coach.name == "David Honea")
        .collect();
    check!(eq;
        context_keys(honea.iter().copied()),
        [
            "David Honea|CrossCountry|Boys",
            "David Honea|CrossCountry|Girls",
            "David Honea|IndoorTrack|Boys",
            "David Honea|OutdoorTrack|Boys",
        ],
        "the captured page publishes one distinct context per team this staff member coaches"
    );
    for coach in &honea {
        check!(eq; coach.role, CoachRole::Unknown);
        check!(eq; coach.phone.as_deref(), Some("(828) 964-6841"));
        check!(eq;
            coach
                .personal_email
                .as_deref()
                .or(coach.professional_email.as_deref()),
            Some("coachhonea@gmail.com")
        );
    }
    let ball = coaches
        .iter()
        .find(|coach| coach.name == "David Ball")
        .ok_or("director row")?;
    check!(eq; ball.sport, None);
    check!(eq; ball.role, CoachRole::AthleticDirector);
    check!(eq; ball.gender, Gender::Mixed);
    check!(eq; ball.phone.as_deref(), Some("(828) 777-7665"));
    let coaches_with_role: usize = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .count();
    check!(eq; coaches_with_role, 2);
    Ok(())
}

#[test]
fn a_summary_without_staff_is_school_identity_only() -> TestResult {
    let summary = parse_summary(IN_SUMMARY.as_bytes())?;
    check!(eq; summary.name, "Muncie Central High School");
    check!(eq; summary.short_code, "QWUGX2");
    check!(summary.staff.is_empty());
    let (_, school_id) = minted(UsJurisdiction::Indiana, "Muncie Central High School");
    let coaches = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/QWUGX2/summary",
    )?
    .coaches;
    check!(coaches.is_empty());
    check!(eq; summary.address.city.as_deref(), Some("Muncie"));
    check!(eq; summary.address.zip.as_deref(), Some("47305"));
    Ok(())
}

#[test]
fn a_body_that_is_not_json_is_a_failed_fetch() {
    assert!(parse_summary(ACCESS_DENIED.as_bytes()).is_err());
    assert!(parse_directory(ACCESS_DENIED.as_bytes()).is_err());
}

#[test]
fn the_summary_fills_what_a_levels_less_directory_row_lacked() -> TestResult {
    let page = parse_directory(GA_DIRECTORY.as_bytes())?;
    let row = page.results.first().ok_or("first school")?;
    let (mut school, _) = admitted(
        UsJurisdiction::Georgia,
        "GHSA",
        row,
        "https://example.test/states/GHSA/directory/2",
    )?;
    check!(eq; school.city.as_deref(), Some("Brunswick"));
    check!(eq; school.classification, None);
    let summary = parse_summary(NC_SUMMARY.as_bytes())?;
    absorb_summary(
        &mut school,
        &summary,
        "https://example.test/schools/ZCUM49/summary",
        OBSERVED_ON,
    );
    check!(eq; school.classification.as_deref(), Some("6A"));
    check!(eq;
        school.aliases,
        vec!["A.C. Reynolds High School".to_string()]
    );
    check!(eq; school.evidence.len(), 2);
    Ok(())
}

#[test]
fn the_possessive_and_genderless_labels_all_map() {
    assert_eq!(
        team_sport("Boys' Track, Outdoor"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        team_sport("Boy's Track, Outdoor"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        team_sport("Girls' Track, Indoor"),
        Some((Sport::IndoorTrack, Gender::Girls))
    );
    assert_eq!(
        team_sport("Girl's Cross Country"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        team_sport("Mixed Track, Outdoor"),
        Some((Sport::OutdoorTrack, Gender::Mixed))
    );
    assert_eq!(
        team_sport("Unified Track, Indoor"),
        Some((Sport::IndoorTrack, Gender::Mixed))
    );
    assert_eq!(
        team_sport("Track, Outdoor"),
        Some((Sport::OutdoorTrack, Gender::Unknown)),
        "an unqualified label must not claim both sides"
    );
    assert_eq!(
        team_sport("Cross Country"),
        Some((Sport::CrossCountry, Gender::Unknown))
    );
    assert_eq!(
        team_sport("Track Cycling"),
        None,
        "cycling is not track and field"
    );
    assert_eq!(team_sport("Girls' Lacrosse"), None);
    assert_eq!(team_sport(""), None);
    assert_eq!(
        team_sport("Unified Mixed Track, Outdoor"),
        None,
        "the prototype strips one genderless prefix and then requires a sport"
    );
}

#[test]
fn a_stated_former_role_never_classifies_as_a_current_one() {
    assert_eq!(coach_role("Former Head Coach"), CoachRole::Unknown);
    assert_eq!(coach_role("Former Assistant Coach"), CoachRole::Unknown);
    assert_eq!(coach_role("Head Coach"), CoachRole::HeadCoach);
    assert_eq!(coach_role("Assistant Coach"), CoachRole::AssistantCoach);
    assert!(!is_director("Former Athletic Director"));
    assert!(is_director("Athletic Director"));
}

#[test]
fn every_label_in_the_prototypes_measured_table_maps() {
    let measured = [
        ("Girls' Track, Outdoor", Sport::OutdoorTrack, Gender::Girls),
        ("Boys' Track, Outdoor", Sport::OutdoorTrack, Gender::Boys),
        ("Girls' Cross Country", Sport::CrossCountry, Gender::Girls),
        ("Boys' Cross Country", Sport::CrossCountry, Gender::Boys),
        ("Girls' Track, Indoor", Sport::IndoorTrack, Gender::Girls),
        ("Boys' Track, Indoor", Sport::IndoorTrack, Gender::Boys),
        ("Girl's Track, Outdoor", Sport::OutdoorTrack, Gender::Girls),
        ("Boy's Track, Outdoor", Sport::OutdoorTrack, Gender::Boys),
        ("Girl's Cross Country", Sport::CrossCountry, Gender::Girls),
        ("Boy's Cross Country", Sport::CrossCountry, Gender::Boys),
        ("Girl's Track, Indoor", Sport::IndoorTrack, Gender::Girls),
        ("Boy's Track, Indoor", Sport::IndoorTrack, Gender::Boys),
        ("Mixed Track, Outdoor", Sport::OutdoorTrack, Gender::Mixed),
        ("Mixed Cross Country", Sport::CrossCountry, Gender::Mixed),
        ("Mixed Track, Indoor", Sport::IndoorTrack, Gender::Mixed),
        ("Unified Track, Outdoor", Sport::OutdoorTrack, Gender::Mixed),
        ("Unified Track, Indoor", Sport::IndoorTrack, Gender::Mixed),
    ];
    for (label, sport, gender) in measured {
        assert_eq!(team_sport(label), Some((sport, gender)), "{label}");
    }
    for label in [
        "Football",
        "Mixed Cheerleading",
        "Boys' Basketball",
        "",
        "Crossfit",
    ] {
        assert_eq!(team_sport(label), None, "{label}");
    }
}

#[test]
fn a_coach_missing_from_the_team_index_is_placed_from_their_own_team() -> TestResult {
    let body = r#"{"name":"Example High School","teams":[{"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]}],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Head Coach","teamName":"Girls' Cross Country","teamLevel":"Varsity"},
        {"id":"b","amrId":"2","firstName":"Grace","lastName":"Hopper","title":"Assistant Coach","teamName":"Boys' Cross Country","teamLevel":"Junior High"},
        {"id":"c","amrId":"3","firstName":"Jean","lastName":"Bartik","title":"Coach","teamName":"Girls' Lacrosse","teamLevel":"Varsity"}
    ]}"#;
    let summary = parse_summary(body.as_bytes())?;
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )?;
    let mut keys: Vec<String> = emission
        .coaches
        .iter()
        .map(|coach| {
            format!(
                "{}|{}|{}|{}",
                coach.name,
                coach.sport.map_or("none", Sport::stable_key),
                coach.gender.stable_key(),
                coach.role.stable_key()
            )
        })
        .collect();
    keys.sort();
    check!(eq; keys, ["Ada Lovelace|OutdoorTrack|Boys|HeadCoach"]);
    check!(eq;
        emission.counters.dropped_levels.get("Junior High"),
        Some(&1),
        "the junior-high staff page is out of census scope (ADR-016 S13)"
    );
    check!(eq; emission.counters.dropped_total(), 1);
    Ok(())
}

#[test]
fn a_nameless_team_member_counts_for_the_probe_and_is_dropped_from_the_census() -> TestResult {
    let body = r#"{"name":"Example High School","teams":[
        {"name":"Girls' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]}
    ],"staff":[
        {"id":"a","amrId":"1","firstName":"","lastName":"","title":"Head Coach","teamName":"","teamLevel":""}
    ]}"#;
    let summary = parse_summary(body.as_bytes())?;
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let probe = super::map::probe_coach_entities(
        &summary,
        &school_id,
        super::map::Capture {
            url: "https://example.test/schools/EX/summary",
            observed_on: OBSERVED_ON,
            sha256: &crate::net::cache::content_digest(body.as_bytes()),
        },
    )?;
    check!(eq;
        probe.coaches.len(),
        1,
        "the prototype probe counts every row the parser returns, named or not"
    );
    let census = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )?;
    check!(census.coaches.is_empty());
    check!(eq; census.counters.dropped_person, 1);
    Ok(())
}

#[test]
fn a_director_who_also_coaches_keeps_both_rows_and_duplicates_collapse() -> TestResult {
    let body = r#"{"name":"Example High School","teams":[
        {"name":"Girls' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]},
        {"name":"Girls' Track, Outdoor","level":"Junior Varsity","coachProfileIds":["a"]}
    ],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Head Coach","teamName":"","teamLevel":"","emails":["ada@example.org"],"tel":[{"num":"555-0100"}]},
        {"id":"b","amrId":"2","firstName":"Grace","lastName":"Hopper","title":"District Athletic Director","teamName":"Boys' Cross Country","teamLevel":"Varsity"}
    ]}"#;
    let summary = parse_summary(body.as_bytes())?;
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let coaches = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )?
    .coaches;
    let mut keys: Vec<String> = coaches
        .iter()
        .map(|coach| {
            format!(
                "{}|{}|{}|{}",
                coach.name,
                coach.sport.map_or("none", Sport::stable_key),
                coach.gender.stable_key(),
                coach.role.stable_key()
            )
        })
        .collect();
    keys.sort();
    check!(eq;
        keys,
        [
            "Ada Lovelace|OutdoorTrack|Girls|HeadCoach",
            "Grace Hopper|CrossCountry|Boys|Unknown",
            "Grace Hopper|none|Mixed|AthleticDirector",
        ]
    );

    let ada = coaches
        .iter()
        .find(|coach| coach.name == "Ada Lovelace")
        .ok_or("coach row")?;
    check!(eq; ada.professional_email.as_deref(), Some("ada@example.org"));
    Ok(())
}

#[test]
fn only_the_measured_associations_are_selectable() {
    assert_eq!(ruleset(UsJurisdiction::NorthCarolina), Some("NCHSAA"));
    assert_eq!(ruleset(UsJurisdiction::Wyoming), Some("WHSAA"));
    assert_eq!(ruleset(UsJurisdiction::DistrictOfColumbia), Some("DCSAA"));
    assert_eq!(ruleset(UsJurisdiction::California), None);
    assert_eq!(ruleset(UsJurisdiction::Indiana), None);
    assert_eq!(ruleset(UsJurisdiction::Connecticut), None);
    assert_eq!(REGISTERED.len(), 15);
    assert_eq!(
        directory_page_url("NCHSAA", 1),
        "https://maxinfosite-api-live.dragonflyathletics.com/states/NCHSAA/directory/1"
    );
    assert_eq!(
        summary_url("ZCUM49"),
        "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary"
    );
}

#[test]
fn the_classification_rule_reads_the_associations_own_key() -> TestResult {
    let levels: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(
        r#"{"nchsaaConference":"Mountain 5A/6A","nchsaaClassifications":"4A","unrelated":7,"nchsaaClassification":"6A"}"#,
    )?;
    check!(eq; classification(&levels).as_deref(), Some("6A"));
    let ahsaa: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(
        r#"{"ahsaaArea":"3","ahsaaClass":"5A","ahsaaDistrict":"1","ahsaaSuper Section":"North"}"#,
    )?;
    check!(eq;
        classification(&ahsaa).as_deref(),
        Some("5A"),
        "the live AHSAA pages name the class without the word classification"
    );
    let whsaa: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(
        r#"{"whsaaClassification":"2A","whsaaMusicDistrict":"East","whsaaRegion":"West"}"#,
    )?;
    check!(eq; classification(&whsaa).as_deref(), Some("2A"));
    let ignored: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(r#"{"nchsaaAlignment":"East","nchsaaCounty":"Wake","class":true}"#)?;
    check!(eq; classification(&ignored), None);
    let none: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(r#"{"region":"8"}"#)?;
    check!(eq; classification(&none), None);
    let empty: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(r#"{"class":"  "}"#)?;
    check!(eq; classification(&empty), None);
    Ok(())
}

#[test]
fn a_request_outside_the_registered_jurisdictions_fetches_nothing() {
    let requested = requested_states(&Options {
        states: vec![UsJurisdiction::California, UsJurisdiction::Indiana],
        ..Options::default()
    });
    assert!(requested.is_empty());
    let all = requested_states(&Options::default());
    assert_eq!(all.len(), 15);
    let some = requested_states(&Options {
        states: vec![UsJurisdiction::NorthCarolina],
        ..Options::default()
    });
    assert_eq!(some, vec![(UsJurisdiction::NorthCarolina, "NCHSAA")]);
}

fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) -> TestResult {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-29T12:00:00Z",
    });
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(cache_dir.join(format!("{key}.body")), body)?;
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta)?,
    )?;
    Ok(())
}

#[test]
fn collect_stores_the_requested_school_and_its_coach_rows_from_the_cache() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    seed_cache(&cache, &directory_page_url("NCHSAA", 1), NC_DIRECTORY)?;
    seed_cache(&cache, &summary_url("ZCUM49"), NC_SUMMARY)?;

    let store = census_store::Store::open(dir.path().join("store"))?;
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    let ctx = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("valid fixture school year")?,
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    };
    let options = Options {
        limit: Some(1),
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: vec![UsJurisdiction::NorthCarolina],
        school_names: vec!["A.C. Reynolds High School".to_string()],
    };
    let report = collect(&ctx, &options).await?;

    check!(eq;
        report.rows, 1,
        "one school processed, the page's rest left to the next run"
    );
    check!(eq; report.errors, 0);
    check!(eq;
        report.requests, 0,
        "both responses came from the seeded cache"
    );
    check!(eq;
        report.from_cache, 2,
        "the directory page and the one school summary"
    );
    check!(eq;
        report.with_email, 16,
        "every published row context carries the staff address"
    );
    check!(
        report
            .notes
            .iter()
            .any(|note| note.contains("coach row(s)")),
        "the report states what it processed: {:?}",
        report.notes
    );

    let schools = store.scan::<CanonicalSchool>(census_store::Table::Schools)?;
    check!(eq; schools.len(), 1);
    let school = schools.first().ok_or("stored requested school")?;
    check!(eq; school.name, "A.C. Reynolds High School");
    check!(eq; school.state, Some(UsJurisdiction::NorthCarolina));
    check!(eq; school.association.as_deref(), Some("NCHSAA"));
    check!(eq; school.city.as_deref(), Some("Asheville"));

    let coaches = store.scan::<census_domain::model::CanonicalCoach>(census_store::Table::Coaches)?;
    check!(eq;
        context_keys(coaches.iter()),
        current_contexts("nc_staff_summary_zcum49.json")?,
        "the stored rows keep every distinct (person, sport family, gender) the page publishes"
    );
    Ok(())
    })
}

#[test]
fn a_live_vendor_fixture_school_row_is_dropped_before_it_mints() -> TestResult {
    let page = parse_directory(NC_DIRECTORY.as_bytes())?;
    let row = page
        .results
        .iter()
        .find(|row| {
            row.name
                .as_deref()
                .is_some_and(|name| name.contains("Test School"))
        })
        .ok_or("the live NC page carries a vendor fixture school")?;
    let admission = directory_school(
        UsJurisdiction::NorthCarolina,
        "NCHSAA",
        row,
        "https://example.test/states/NCHSAA/directory/1",
        OBSERVED_ON,
    )?;
    check!(eq;
        admission,
        DirectoryAdmission::DroppedName,
        "a vendor fixture school never reaches the store (ADR-016 S04)"
    );
    Ok(())
}

#[test]
fn a_vendor_fixture_contact_drops_the_coach_row() -> TestResult {
    let body = r#"{"name":"Example High School","teams":[{"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a","b"]}],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity","emails":["ada@school.org"]},
        {"id":"b","amrId":"2","firstName":"Dana","lastName":"Vendor","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity","emails":["d@dragonflyathletics.com"]}
    ]}"#;
    let summary = parse_summary(body.as_bytes())?;
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )?;
    let names: Vec<&str> = emission
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    check!(eq; names, ["Ada Lovelace"]);
    check!(eq; emission.counters.dropped_vendor, 1);
    check!(eq; emission.counters.dropped_total(), 0);
    Ok(())
}

#[test]
fn a_sub_varsity_team_row_is_dropped_and_counted_per_level() -> TestResult {
    let body = r#"{"name":"Example High School","teams":[
        {"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]},
        {"name":"Girls' Track, Outdoor","level":"Junior Varsity","coachProfileIds":["b"]},
        {"name":"Girls' Track, Outdoor","level":"Junior High","coachProfileIds":["c"]}
    ],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"b","amrId":"2","firstName":"Grace","lastName":"Hopper","title":"Coach","teamName":"Girls' Track, Outdoor","teamLevel":"Junior Varsity"},
        {"id":"c","amrId":"3","firstName":"Jean","lastName":"Bartik","title":"Coach","teamName":"Girls' Track, Outdoor","teamLevel":"Junior High"}
    ]}"#;
    let summary = parse_summary(body.as_bytes())?;
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )?;
    let names: Vec<&str> = emission
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    check!(eq;
        names,
        ["Ada Lovelace"],
        "only the varsity team row is in census scope (ADR-016 S12)"
    );
    check!(eq;
        emission.counters.dropped_levels.get("Junior Varsity"),
        Some(&1)
    );
    check!(eq;
        emission.counters.dropped_levels.get("Junior High"),
        Some(&1)
    );
    check!(eq; emission.counters.dropped_total(), 2);
    Ok(())
}

#[test]
fn a_live_middle_school_page_keeps_only_its_director_and_counts_each_level() -> TestResult {
    let summary = parse_summary(AL_MIDDLE_SCHOOL_SUMMARY.as_bytes())?;
    check!(eq; summary.name, "Rainbow Middle School");
    let (_, school_id) = minted(UsJurisdiction::Alabama, "Rainbow Middle School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/SVXJDF/summary",
    )?;
    let rows: Vec<(&str, CoachRole)> = emission
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.role))
        .collect();
    check!(eq;
        rows,
        [("Allison Lee", CoachRole::AthleticDirector)],
        "every track row on the live page is a middle-school row (ADR-016 S12)"
    );
    check!(eq;
        emission.counters.dropped_levels.get("Middle School"),
        Some(&6),
        "two cross country coaches twice, one track coach once per gender"
    );
    check!(eq; emission.counters.dropped_total(), 6);
    check!(eq; emission.counters.dropped_person, 0);
    check!(eq; emission.counters.dropped_vendor, 0);
    Ok(())
}

#[test]
fn the_live_directory_pages_reproduce_the_prototypes_rows() -> TestResult {
    let golden: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
        serde_json::from_str(GOLDEN_DIRECTORY_ROWS)?;
    let fixtures: [(&str, &str); 3] = [
        ("probe/AK/directory-1.json", AK_DIRECTORY),
        ("probe/WY/directory-1.json", WY_DIRECTORY),
        ("nchsaa_directory_p1.json", NC_DIRECTORY),
    ];
    let mut rows_checked = 0usize;
    for (key, body) in fixtures {
        let expected = golden.get(key).ok_or("page has golden rows")?;
        let page = parse_directory(body.as_bytes())?;
        check!(eq;
            page.results.len(),
            expected.len(),
            "{key}: the prototype mapped every published row"
        );
        for (position, (row, want)) in page.results.iter().zip(expected).enumerate() {
            let levels: std::collections::BTreeMap<String, serde_json::Value> =
                serde_json::from_value(want["dragonfly_levels"].clone())?;
            check!(eq;
                row.competition_levels, levels,
                "{key} row {position}: dragonfly_levels differ"
            );
            let code = row.short_code.as_deref().map_or("", |value| value);
            let got = [
                ("name", row.name.as_deref().map_or("", |value| value)),
                ("state", row.state_code.as_deref().map_or("", |value| value)),
                ("city", row.city.as_deref().map_or("", |value| value)),
                ("address", row.address.as_deref().map_or("", |value| value)),
                (
                    "association_id",
                    row.org_id.as_deref().map_or("", |value| value),
                ),
                ("dragonfly_short_code", code),
            ];
            for (field, value) in got {
                check!(eq;
                    value,
                    want[field].as_str().map_or("", |value| value),
                    "{key} row {position}: {field} differs"
                );
            }
            let detail = if code.is_empty() {
                String::new()
            } else {
                format!("{API_HOST}/schools/{code}/summary")
            };
            check!(eq;
                detail,
                want["detail_url"].as_str().map_or("", |value| value),
                "{key} row {position}: detail_url differs"
            );
            rows_checked += 1;
        }
    }
    check!(eq; rows_checked, 900, "the three captured pages, row for row");
    Ok(())
}

#[test]
fn a_post_only_or_unnamed_staff_string_never_mints_a_person() -> TestResult {
    let body = r#"{"name":"Example High School","teams":[{"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a","b","c","d"]}],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"b","amrId":"2","firstName":"Head","lastName":"Coach","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"c","amrId":"3","firstName":"Principal","lastName":"Kolling","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"d","amrId":"4","firstName":"","lastName":"","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"}
    ]}"#;
    let summary = parse_summary(body.as_bytes())?;
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )?;
    let names: Vec<&str> = emission
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    check!(eq; names, ["Ada Lovelace"]);
    check!(eq;
        emission.counters.dropped_person, 3,
        "two post-only leads and the unnamed member never mint a person (ADR-016 S03/S09)"
    );
    check!(eq; emission.counters.dropped_total(), 0);
    Ok(())
}

mod claim_order;
