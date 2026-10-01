mod summary_parity;
use super::collect::requested_states;
use super::map::{absorb_summary, coach_entities, directory_school, team_sport};
use super::parse::{parse_directory, parse_summary};
use super::*;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport,
};
use census_domain::UsJurisdiction;

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

const OBSERVED_ON: &str = "2026-09-29";

const NC_SUMMARY_CONTEXTS: [&str; 16] = [
    "Amelia Rogers-roper|OutdoorTrack|Boys",
    "Amelia Rogers-roper|OutdoorTrack|Girls",
    "Andy Morgan|IndoorTrack|Boys",
    "Andy Morgan|IndoorTrack|Girls",
    "Andy Morgan|OutdoorTrack|Boys",
    "Andy Morgan|OutdoorTrack|Girls",
    "David Ball|none|Mixed",
    "David Honea|CrossCountry|Boys",
    "David Honea|CrossCountry|Girls",
    "David Honea|IndoorTrack|Boys",
    "David Honea|OutdoorTrack|Boys",
    "Ivy Briggs|CrossCountry|Girls",
    "Maura Brouwer|OutdoorTrack|Girls",
    "Rocky Bilotta|OutdoorTrack|Boys",
    "Steve Mccurry|none|Mixed",
    "William Greer|OutdoorTrack|Boys",
];

fn minted(state: UsJurisdiction, name: &str) -> (CanonicalSchool, census_domain::model::SchoolId) {
    CanonicalSchool::new(state, name, normalize_name(name))
}

fn admitted(
    state: UsJurisdiction,
    association: &str,
    row: &DirectorySchool,
    url: &str,
) -> (CanonicalSchool, census_domain::model::SchoolId) {
    match directory_school(state, association, row, url, OBSERVED_ON).expect("directory row") {
        DirectoryAdmission::School(school, id) => (*school, id),
        other => panic!("directory row not admitted: {other:?}"),
    }
}

fn emitted(
    summary: &SchoolSummary,
    school_id: &census_domain::model::SchoolId,
    url: &str,
) -> CoachEmission {
    coach_entities(summary, school_id, url, OBSERVED_ON, EmissionScope::Census).expect("coach rows")
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
fn a_directory_page_reads_every_association_row() {
    let page = parse_directory(NC_DIRECTORY.as_bytes()).expect("directory page");
    assert_eq!(page.current_page, 1);
    assert_eq!(page.total_pages, 1);
    assert_eq!(page.total_results, 452);
    assert_eq!(page.results.len(), 452);
    let row = page.results.first().expect("first school");
    assert_eq!(row.name.as_deref(), Some("A.C. Reynolds High School"));
    assert_eq!(row.short_code.as_deref(), Some("ZCUM49"));
    assert_eq!(row.org_id.as_deref(), Some("6285bff87f770402d0000006"));
    assert_eq!(row.city.as_deref(), Some("Asheville"));
    assert_eq!(row.address.as_deref(), Some("1 Rocket Drive"));
    assert_eq!(
        classification(&row.competition_levels).as_deref(),
        Some("6A")
    );
}

#[test]
fn a_page_without_association_levels_still_reads() {
    let page = parse_directory(GA_DIRECTORY.as_bytes()).expect("directory page");
    assert_eq!(page.current_page, 2);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.results.len(), 1000);
    let row = page.results.first().expect("first school");
    assert_eq!(row.name.as_deref(), Some("Glynn Middle"));
    assert_eq!(row.state_code.as_deref(), Some("GA"));
    assert!(row.competition_levels.is_empty());
    assert_eq!(classification(&row.competition_levels), None);
}

#[test]
fn the_directory_row_mints_the_school_with_its_association_and_level() {
    let page = parse_directory(NC_DIRECTORY.as_bytes()).expect("directory page");
    let row = page.results.first().expect("first school");
    let (school, id) = admitted(
        UsJurisdiction::NorthCarolina,
        "NCHSAA",
        row,
        "https://example.test/states/NCHSAA/directory/1",
    );
    assert_eq!(school.name, "A.C. Reynolds High School");
    assert_eq!(school.city.as_deref(), Some("Asheville"));
    assert_eq!(school.association.as_deref(), Some("NCHSAA"));
    assert_eq!(school.classification.as_deref(), Some("6A"));
    assert_eq!(school.state, Some(UsJurisdiction::NorthCarolina));
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(school.evidence.len(), 1);
    assert_eq!(
        id.as_str(),
        CanonicalSchool::mint(
            UsJurisdiction::NorthCarolina,
            "A.C. Reynolds High School",
            &normalize_name("A.C. Reynolds High School")
        )
        .as_str()
    );
}

#[test]
fn the_summary_maps_every_published_row() {
    let summary = parse_summary(NC_SUMMARY.as_bytes()).expect("summary");
    assert_eq!(summary.short_code, "ZCUM49");
    let (_, school_id) = minted(UsJurisdiction::NorthCarolina, "A.C. Reynolds High School");
    let coaches = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/ZCUM49/summary",
    )
    .coaches;
    assert_eq!(context_keys(coaches.iter()), NC_SUMMARY_CONTEXTS);
    let honea: Vec<&CanonicalCoach> = coaches
        .iter()
        .filter(|coach| coach.name == "David Honea")
        .collect();
    assert_eq!(
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
        assert_eq!(coach.role, CoachRole::Unknown);
        assert_eq!(coach.phone.as_deref(), Some("(828) 964-6841"));
        assert_eq!(
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
        .expect("director row");
    assert_eq!(ball.sport, None);
    assert_eq!(ball.role, CoachRole::AthleticDirector);
    assert_eq!(ball.gender, Gender::Mixed);
    assert_eq!(ball.phone.as_deref(), Some("(828) 777-7665"));
    let coaches_with_role: usize = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .count();
    assert_eq!(coaches_with_role, 2);
}

#[test]
fn a_summary_without_staff_is_school_identity_only() {
    let summary = parse_summary(IN_SUMMARY.as_bytes()).expect("summary");
    assert_eq!(summary.name, "Muncie Central High School");
    assert_eq!(summary.short_code, "QWUGX2");
    assert!(summary.staff.is_empty());
    let (_, school_id) = minted(UsJurisdiction::Indiana, "Muncie Central High School");
    let coaches = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/QWUGX2/summary",
    )
    .coaches;
    assert!(coaches.is_empty());
    assert_eq!(summary.address.city.as_deref(), Some("Muncie"));
    assert_eq!(summary.address.zip.as_deref(), Some("47305"));
}

#[test]
fn a_body_that_is_not_json_is_a_failed_fetch() {
    assert!(parse_summary(ACCESS_DENIED.as_bytes()).is_err());
    assert!(parse_directory(ACCESS_DENIED.as_bytes()).is_err());
}

#[test]
fn the_summary_fills_what_a_levels_less_directory_row_lacked() {
    let page = parse_directory(GA_DIRECTORY.as_bytes()).expect("directory page");
    let row = page.results.first().expect("first school");
    let (mut school, _) = admitted(
        UsJurisdiction::Georgia,
        "GHSA",
        row,
        "https://example.test/states/GHSA/directory/2",
    );
    assert_eq!(school.city.as_deref(), Some("Brunswick"));
    assert_eq!(school.classification, None);
    let summary = parse_summary(NC_SUMMARY.as_bytes()).expect("summary");
    absorb_summary(
        &mut school,
        &summary,
        "https://example.test/schools/ZCUM49/summary",
        OBSERVED_ON,
    );
    assert_eq!(school.classification.as_deref(), Some("6A"));
    assert_eq!(
        school.aliases,
        vec!["A.C. Reynolds High School".to_string()]
    );
    assert_eq!(school.evidence.len(), 2);
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
    assert_eq!(team_sport("Girls' Lacrosse"), None);
    assert_eq!(team_sport(""), None);
    assert_eq!(
        team_sport("Unified Mixed Track, Outdoor"),
        None,
        "the prototype strips one genderless prefix and then requires a sport"
    );
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
fn a_coach_missing_from_the_team_index_is_placed_from_their_own_team() {
    let body = r#"{"name":"Example High School","teams":[{"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]}],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Head Coach","teamName":"Girls' Cross Country","teamLevel":"Varsity"},
        {"id":"b","amrId":"2","firstName":"Grace","lastName":"Hopper","title":"Assistant Coach","teamName":"Boys' Cross Country","teamLevel":"Junior High"},
        {"id":"c","amrId":"3","firstName":"Jean","lastName":"Bartik","title":"Coach","teamName":"Girls' Lacrosse","teamLevel":"Varsity"}
    ]}"#;
    let summary = parse_summary(body.as_bytes()).expect("summary");
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    );
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
    assert_eq!(keys, ["Ada Lovelace|OutdoorTrack|Boys|HeadCoach"]);
    assert_eq!(
        emission.counters.dropped_levels.get("Junior High"),
        Some(&1),
        "the junior-high staff page is out of census scope (ADR-016 S13)"
    );
    assert_eq!(emission.counters.dropped_total(), 1);
}

#[test]
fn a_nameless_team_member_counts_for_the_probe_and_is_dropped_from_the_census() {
    let body = r#"{"name":"Example High School","teams":[
        {"name":"Girls' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]}
    ],"staff":[
        {"id":"a","amrId":"1","firstName":"","lastName":"","title":"Head Coach","teamName":"","teamLevel":""}
    ]}"#;
    let summary = parse_summary(body.as_bytes()).expect("summary");
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let probe = coach_entities(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
        OBSERVED_ON,
        EmissionScope::Probe,
    )
    .expect("probe rows");
    assert_eq!(
        probe.coaches.len(),
        1,
        "the prototype probe counts every row the parser returns, named or not"
    );
    let census = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    );
    assert!(census.coaches.is_empty());
    assert_eq!(census.counters.dropped_person, 1);
}

#[test]
fn a_director_who_also_coaches_keeps_both_rows_and_duplicates_collapse() {
    let body = r#"{"name":"Example High School","teams":[
        {"name":"Girls' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]},
        {"name":"Girls' Track, Outdoor","level":"Junior Varsity","coachProfileIds":["a"]}
    ],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Head Coach","teamName":"","teamLevel":"","emails":["ada@example.org"],"tel":[{"num":"555-0100"}]},
        {"id":"b","amrId":"2","firstName":"Grace","lastName":"Hopper","title":"District Athletic Director","teamName":"Boys' Cross Country","teamLevel":"Varsity"}
    ]}"#;
    let summary = parse_summary(body.as_bytes()).expect("summary");
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let coaches = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    )
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
    assert_eq!(
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
        .expect("coach row");
    assert_eq!(ada.professional_email.as_deref(), Some("ada@example.org"));
}

#[test]
fn only_the_measured_associations_are_selectable() {
    assert_eq!(ruleset(UsJurisdiction::NorthCarolina), Some("NCHSAA"));
    assert_eq!(ruleset(UsJurisdiction::Wyoming), Some("WHSAA"));
    assert_eq!(ruleset(UsJurisdiction::DistrictOfColumbia), Some("DCSAA"));
    assert_eq!(ruleset(UsJurisdiction::California), None);
    assert_eq!(ruleset(UsJurisdiction::Indiana), None);
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
fn the_classification_rule_reads_the_associations_own_key() {
    let levels: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(
        r#"{"nchsaaConference":"Mountain 5A/6A","nchsaaClassifications":"4A","unrelated":7,"nchsaaClassification":"6A"}"#,
    )
    .expect("levels");
    assert_eq!(classification(&levels).as_deref(), Some("6A"));
    let ahsaa: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(
        r#"{"ahsaaArea":"3","ahsaaClass":"5A","ahsaaDistrict":"1","ahsaaSuper Section":"North"}"#,
    )
    .expect("levels");
    assert_eq!(
        classification(&ahsaa).as_deref(),
        Some("5A"),
        "the live AHSAA pages name the class without the word classification"
    );
    let whsaa: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(
        r#"{"whsaaClassification":"2A","whsaaMusicDistrict":"East","whsaaRegion":"West"}"#,
    )
    .expect("levels");
    assert_eq!(classification(&whsaa).as_deref(), Some("2A"));
    let ignored: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(r#"{"nchsaaAlignment":"East","nchsaaCounty":"Wake","class":true}"#)
            .expect("levels");
    assert_eq!(classification(&ignored), None);
    let none: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(r#"{"region":"8"}"#).expect("levels");
    assert_eq!(classification(&none), None);
    let empty: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(r#"{"class":"  "}"#).expect("levels");
    assert_eq!(classification(&empty), None);
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

fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) {
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
    std::fs::create_dir_all(cache_dir).expect("cache dir");
    std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("cache body");
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta).expect("cache meta"),
    )
    .expect("cache meta written");
}

#[tokio::test]
async fn collect_stores_the_requested_school_and_its_coach_rows_from_the_cache() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    seed_cache(&cache, &directory_page_url("NCHSAA", 1), NC_DIRECTORY);
    seed_cache(&cache, &summary_url("ZCUM49"), NC_SUMMARY);

    let store = census_store::Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let ctx = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
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
    let report = collect(&ctx, &options)
        .await
        .expect("collect returns a report");

    assert_eq!(
        report.rows, 1,
        "one school processed, the page's rest left to the next run"
    );
    assert_eq!(report.errors, 0);
    assert_eq!(
        report.requests, 0,
        "both responses came from the seeded cache"
    );
    assert_eq!(
        report.from_cache, 2,
        "the directory page and the one school summary"
    );
    assert_eq!(
        report.with_email, 16,
        "every published row context carries the staff address"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("coach row(s)")),
        "the report states what it processed: {:?}",
        report.notes
    );

    let schools = store
        .scan::<CanonicalSchool>(census_store::Table::Schools)
        .expect("schools log");
    assert_eq!(schools.len(), 1);
    assert_eq!(schools[0].name, "A.C. Reynolds High School");
    assert_eq!(schools[0].state, Some(UsJurisdiction::NorthCarolina));
    assert_eq!(schools[0].association.as_deref(), Some("NCHSAA"));
    assert_eq!(schools[0].city.as_deref(), Some("Asheville"));

    let coaches = store
        .scan::<census_domain::model::CanonicalCoach>(census_store::Table::Coaches)
        .expect("coach rows");
    assert_eq!(
        context_keys(coaches.iter()),
        NC_SUMMARY_CONTEXTS,
        "the stored rows keep every distinct (person, sport family, gender) the page publishes"
    );
}

#[test]
fn a_live_vendor_fixture_school_row_is_dropped_before_it_mints() {
    let page = parse_directory(NC_DIRECTORY.as_bytes()).expect("directory page");
    let row = page
        .results
        .iter()
        .find(|row| {
            row.name
                .as_deref()
                .is_some_and(|name| name.contains("Test School"))
        })
        .expect("the live NC page carries a vendor fixture school");
    let admission = directory_school(
        UsJurisdiction::NorthCarolina,
        "NCHSAA",
        row,
        "https://example.test/states/NCHSAA/directory/1",
        OBSERVED_ON,
    )
    .expect("directory row");
    assert_eq!(
        admission,
        DirectoryAdmission::DroppedName,
        "a vendor fixture school never reaches the store (ADR-016 S04)"
    );
}

#[test]
fn a_vendor_fixture_contact_drops_the_coach_row() {
    let body = r#"{"name":"Example High School","teams":[{"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a","b"]}],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity","emails":["ada@school.org"]},
        {"id":"b","amrId":"2","firstName":"Dana","lastName":"Vendor","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity","emails":["d@dragonflyathletics.com"]}
    ]}"#;
    let summary = parse_summary(body.as_bytes()).expect("summary");
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    );
    let names: Vec<&str> = emission
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(names, ["Ada Lovelace"]);
    assert_eq!(emission.counters.dropped_vendor, 1);
    assert_eq!(emission.counters.dropped_total(), 0);
}

#[test]
fn a_sub_varsity_team_row_is_dropped_and_counted_per_level() {
    let body = r#"{"name":"Example High School","teams":[
        {"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a"]},
        {"name":"Girls' Track, Outdoor","level":"Junior Varsity","coachProfileIds":["b"]},
        {"name":"Girls' Track, Outdoor","level":"Junior High","coachProfileIds":["c"]}
    ],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"b","amrId":"2","firstName":"Grace","lastName":"Hopper","title":"Coach","teamName":"Girls' Track, Outdoor","teamLevel":"Junior Varsity"},
        {"id":"c","amrId":"3","firstName":"Jean","lastName":"Bartik","title":"Coach","teamName":"Girls' Track, Outdoor","teamLevel":"Junior High"}
    ]}"#;
    let summary = parse_summary(body.as_bytes()).expect("summary");
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    );
    let names: Vec<&str> = emission
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Ada Lovelace"],
        "only the varsity team row is in census scope (ADR-016 S12)"
    );
    assert_eq!(
        emission.counters.dropped_levels.get("Junior Varsity"),
        Some(&1)
    );
    assert_eq!(
        emission.counters.dropped_levels.get("Junior High"),
        Some(&1)
    );
    assert_eq!(emission.counters.dropped_total(), 2);
}

#[test]
fn a_live_middle_school_page_keeps_only_its_director_and_counts_each_level() {
    let summary = parse_summary(AL_MIDDLE_SCHOOL_SUMMARY.as_bytes()).expect("summary");
    assert_eq!(summary.name, "Rainbow Middle School");
    let (_, school_id) = minted(UsJurisdiction::Alabama, "Rainbow Middle School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/SVXJDF/summary",
    );
    let rows: Vec<(&str, CoachRole)> = emission
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.role))
        .collect();
    assert_eq!(
        rows,
        [("Allison Lee", CoachRole::AthleticDirector)],
        "every track row on the live page is a middle-school row (ADR-016 S12)"
    );
    assert_eq!(
        emission.counters.dropped_levels.get("Middle School"),
        Some(&6),
        "two cross country coaches twice, one track coach once per gender"
    );
    assert_eq!(emission.counters.dropped_total(), 6);
    assert_eq!(emission.counters.dropped_person, 0);
    assert_eq!(emission.counters.dropped_vendor, 0);
}

#[test]
fn the_live_directory_pages_reproduce_the_prototypes_rows() {
    let golden: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
        serde_json::from_str(GOLDEN_DIRECTORY_ROWS).expect("golden rows");
    let fixtures: [(&str, &str); 3] = [
        ("probe/AK/directory-1.json", AK_DIRECTORY),
        ("probe/WY/directory-1.json", WY_DIRECTORY),
        ("nchsaa_directory_p1.json", NC_DIRECTORY),
    ];
    let mut rows_checked = 0usize;
    for (key, body) in fixtures {
        let expected = golden.get(key).expect("page has golden rows");
        let page = parse_directory(body.as_bytes()).expect("directory page");
        assert_eq!(
            page.results.len(),
            expected.len(),
            "{key}: the prototype mapped every published row"
        );
        for (position, (row, want)) in page.results.iter().zip(expected).enumerate() {
            let levels: std::collections::BTreeMap<String, serde_json::Value> =
                serde_json::from_value(want["dragonfly_levels"].clone()).expect("levels");
            assert_eq!(
                row.competition_levels, levels,
                "{key} row {position}: dragonfly_levels differ"
            );
            let code = row.short_code.as_deref().unwrap_or("");
            let got = [
                ("name", row.name.as_deref().unwrap_or("")),
                ("state", row.state_code.as_deref().unwrap_or("")),
                ("city", row.city.as_deref().unwrap_or("")),
                ("address", row.address.as_deref().unwrap_or("")),
                ("association_id", row.org_id.as_deref().unwrap_or("")),
                ("dragonfly_short_code", code),
            ];
            for (field, value) in got {
                assert_eq!(
                    value,
                    want[field].as_str().unwrap_or(""),
                    "{key} row {position}: {field} differs"
                );
            }
            let detail = if code.is_empty() {
                String::new()
            } else {
                format!("{API_HOST}/schools/{code}/summary")
            };
            assert_eq!(
                detail,
                want["detail_url"].as_str().unwrap_or(""),
                "{key} row {position}: detail_url differs"
            );
            rows_checked += 1;
        }
    }
    assert_eq!(rows_checked, 900, "the three captured pages, row for row");
}

#[test]
fn a_post_only_or_unnamed_staff_string_never_mints_a_person() {
    let body = r#"{"name":"Example High School","teams":[{"name":"Boys' Track, Outdoor","level":"Varsity","coachProfileIds":["a","b","c","d"]}],"staff":[
        {"id":"a","amrId":"1","firstName":"Ada","lastName":"Lovelace","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"b","amrId":"2","firstName":"Head","lastName":"Coach","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"c","amrId":"3","firstName":"Principal","lastName":"Kolling","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"},
        {"id":"d","amrId":"4","firstName":"","lastName":"","title":"Coach","teamName":"Boys' Track, Outdoor","teamLevel":"Varsity"}
    ]}"#;
    let summary = parse_summary(body.as_bytes()).expect("summary");
    let (_, school_id) = minted(UsJurisdiction::Georgia, "Example High School");
    let emission = emitted(
        &summary,
        &school_id,
        "https://example.test/schools/EX/summary",
    );
    let names: Vec<&str> = emission
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(names, ["Ada Lovelace"]);
    assert_eq!(
        emission.counters.dropped_person, 3,
        "two post-only leads and the unnamed member never mint a person (ADR-016 S03/S09)"
    );
    assert_eq!(emission.counters.dropped_total(), 0);
}

mod claim_order;
