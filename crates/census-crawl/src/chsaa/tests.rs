use super::collect::wanted_names;
use super::map::{map_coach_row, map_directory_row};
use super::parse::{parse_directory, parse_school_page};
use super::*;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const DIRECTORY: &str = include_str!("../../tests/fixtures/chsaa/directory.html");
const SCHOOL_PAGE: &str = include_str!("../../tests/fixtures/chsaa/school_cherry_creek.html");
const ROBOTS: &str = include_str!("../../tests/fixtures/chsaa/robots.txt");
const GOLDEN_DIRECTORY: &str =
    include_str!("../../tests/fixtures/chsaa/golden_directory_rows.json");
const GOLDEN_COACHES: &str =
    include_str!("../../tests/fixtures/chsaa/golden_school_coach_rows.json");

const OBSERVED_ON: &str = "2026-09-27";

fn golden_rows(json: &str) -> Vec<Value> {
    serde_json::from_str(json).expect("the golden is a JSON array of the prototype's rows")
}

fn field(row: &Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn sport_key(sport: Sport) -> &'static str {
    match sport {
        Sport::IndoorTrack | Sport::OutdoorTrack => "Track",
        Sport::CrossCountry => "CrossCountry",
    }
}

fn coach_key(coach: &CanonicalCoach) -> (String, String, String, String) {
    (
        coach.name.clone(),
        coach.sport.map(sport_key).unwrap_or("none").to_string(),
        format!("{:?}", coach.role),
        format!("{:?}", coach.gender),
    )
}

fn cherry_creek() -> super::parse::MemberSchool {
    let schools = parse_directory(DIRECTORY).expect("directory parses");
    schools
        .into_iter()
        .find(|school| school.name.as_deref() == Some("Cherry Creek"))
        .expect("the directory holds Cherry Creek")
}

fn cherry_creek_slug() -> String {
    cherry_creek().slug.expect("Cherry Creek rows carry a slug")
}

#[test]
fn directory_parses_the_378_member_schools() {
    let schools = parse_directory(DIRECTORY).expect("directory parses");
    assert_eq!(schools.len(), 378, "the capture holds 378 member schools");
    assert_eq!(schools[0].name.as_deref(), Some("Academy"));
}

#[test]
fn directory_rows_match_the_prototype_golden_field_for_field() {
    let schools = parse_directory(DIRECTORY).expect("directory parses");
    let golden = golden_rows(GOLDEN_DIRECTORY);
    assert_eq!(schools.len(), golden.len());
    for (rust, prototype) in schools.iter().zip(&golden) {
        assert_eq!(rust.name.as_deref(), field(prototype, "name").as_deref());
        assert_eq!(
            rust.official_name.as_deref(),
            field(prototype, "official_name").as_deref()
        );
        assert_eq!(rust.city.as_deref(), field(prototype, "city").as_deref());
        assert_eq!(
            rust.street_address.as_deref(),
            field(prototype, "address").as_deref()
        );
        assert_eq!(rust.zip_code.as_deref(), field(prototype, "zip").as_deref());
        assert_eq!(rust.phone.as_deref(), field(prototype, "phone").as_deref());
        assert_eq!(
            rust.district_name.as_deref(),
            field(prototype, "district").as_deref()
        );
        assert_eq!(
            rust.member_type.as_deref(),
            field(prototype, "member_type").as_deref()
        );
        assert_eq!(
            rust.school_type.as_deref(),
            field(prototype, "school_type").as_deref()
        );
        assert_eq!(
            rust.setting.as_deref(),
            field(prototype, "setting").as_deref()
        );
        assert_eq!(
            rust.school_code.map(|code| code.to_string()).as_deref(),
            field(prototype, "association_id").as_deref(),
            "the prototype's string association_id is the Rust row's numeric school_code"
        );
        let slug = rust.slug.as_deref().expect("every golden row has a slug");
        assert_eq!(
            school_page_url(slug),
            field(prototype, "detail_url").unwrap_or_default(),
            "the slug names the prototype's own school URL"
        );
    }
}

#[test]
fn directory_address_coverage_matches_the_recorded_figures() {
    let schools = parse_directory(DIRECTORY).expect("directory parses");
    let with_address = schools
        .iter()
        .filter(|school| school.street_address.is_some())
        .count();
    let with_zip = schools
        .iter()
        .filter(|school| school.zip_code.is_some())
        .count();
    assert_eq!(
        with_address, 378,
        "the applicability record cites 378/378 addresses"
    );
    assert_eq!(with_zip, 376, "the applicability record cites 376/378 zips");
}

#[test]
fn school_page_rows_match_the_prototype_golden_after_mapping() {
    let member = cherry_creek();
    let (_, school_id) =
        map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).expect("Cherry Creek is a school");
    let url = school_page_url(member.slug.as_deref().unwrap_or_default());
    let rows = parse_school_page(SCHOOL_PAGE).expect("the school page parses");
    let mut rust: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    for row in &rows {
        if let Some(coach) = map_coach_row(row, &school_id, &url, OBSERVED_ON) {
            *rust.entry(coach_key(&coach)).or_insert(0) += 1;
        }
    }
    let prototype = golden_rows(GOLDEN_COACHES);
    let mut expected: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    for row in &prototype {
        if field(row, "sport").as_deref() == Some("Track")
            || field(row, "sport").as_deref() == Some("CrossCountry")
        {
            let key = (
                field(row, "person").unwrap_or_default(),
                field(row, "sport").unwrap_or_default(),
                field(row, "role").unwrap_or_default(),
                field(row, "gender").unwrap_or_default(),
            );
            *expected.entry(key).or_insert(0) += 1;
        }
    }
    assert_eq!(
        rust, expected,
        "the Rust (person, sport, role, gender) set equals the prototype's; the prototype's level column is uniform Varsity here and the census model stores no coach level"
    );
    assert!(
        rows.len() > 50,
        "the page lists every activity it publishes; the mapper is what narrows to track and cross country"
    );
    assert_eq!(rust.values().sum::<usize>(), 50);
}

#[test]
fn school_page_names_the_school_from_its_title() {
    let rows = parse_school_page(SCHOOL_PAGE).expect("the school page parses");
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row.school_name == "Cherry Creek"));
}

#[test]
fn mapped_coaches_are_never_athletic_directors() {
    let member = cherry_creek();
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).expect("school");
    for row in parse_school_page(SCHOOL_PAGE).expect("the school page parses") {
        if let Some(coach) = map_coach_row(&row, &school_id, DIRECTORY_URL, OBSERVED_ON) {
            assert_ne!(coach.role, CoachRole::AthleticDirector);
        }
    }
}

#[test]
fn sports_outside_track_and_cross_country_are_dropped() {
    let member = cherry_creek();
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).expect("school");
    let row = super::parse::SchoolCoachRow {
        school_name: "Cherry Creek".to_string(),
        person: "Someone".to_string(),
        activity_name: "Boys Basketball".to_string(),
        title: "Head Coach".to_string(),
    };
    assert!(map_coach_row(&row, &school_id, DIRECTORY_URL, OBSERVED_ON).is_none());
}

#[test]
fn a_role_outside_the_coaching_titles_is_dropped() {
    let member = cherry_creek();
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).expect("school");
    let row = super::parse::SchoolCoachRow {
        school_name: "Cherry Creek".to_string(),
        person: "Someone".to_string(),
        activity_name: "Boys Cross Country".to_string(),
        title: "Volunteer".to_string(),
    };
    assert!(map_coach_row(&row, &school_id, DIRECTORY_URL, OBSERVED_ON).is_none());
}

#[test]
fn gender_comes_from_the_activity_name() {
    let member = cherry_creek();
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).expect("school");
    let boys = super::parse::SchoolCoachRow {
        school_name: "Cherry Creek".to_string(),
        person: "A".to_string(),
        activity_name: "Boys Cross Country".to_string(),
        title: "Head Coach".to_string(),
    };
    let girls = super::parse::SchoolCoachRow {
        school_name: "Cherry Creek".to_string(),
        person: "B".to_string(),
        activity_name: "Girls Track and Field".to_string(),
        title: "Assistant Coach".to_string(),
    };
    assert_eq!(
        map_coach_row(&boys, &school_id, DIRECTORY_URL, OBSERVED_ON).map(|coach| coach.gender),
        Some(Gender::Boys)
    );
    let coach = map_coach_row(&girls, &school_id, DIRECTORY_URL, OBSERVED_ON).expect("girls row");
    assert_eq!(coach.gender, Gender::Girls);
    assert_eq!(coach.sport, Some(Sport::OutdoorTrack));
}

#[test]
fn malformed_directory_errors_instead_of_panicking() {
    assert!(parse_directory("<html><body>no member array</body></html>").is_err());
    assert!(parse_directory("").is_err());
    assert!(parse_directory("[{\"schoolCode\"").is_err());
}

#[test]
fn malformed_school_page_errors_instead_of_panicking() {
    assert!(parse_school_page("<html><body>no activities</body></html>").is_err());
    assert!(parse_school_page("").is_err());
}

#[test]
fn descriptor_is_registered_with_the_directory_capabilities() {
    let entry = crate::registry::descriptor("chsaa").expect("chsaa is registered");
    assert_eq!(entry.transport, crate::registry::TransportKind::Html);
    assert!(entry.capabilities.school_evidence);
    assert!(entry.capabilities.coach_directory);
    assert!(!entry.capabilities.public_professional_contact);
}

#[test]
fn robots_allows_the_member_directory_path() {
    assert!(ROBOTS.contains("User-agent: *"));
    assert!(ROBOTS.contains("Allow: /"));
    assert!(
        ROBOTS
            .lines()
            .filter(|line| line.starts_with("Disallow:"))
            .all(|line| !line.contains("/schools")),
        "no Disallow rule covers /schools"
    );
    assert!(
        ROBOTS
            .lines()
            .filter(|line| line.starts_with("Crawl-delay:"))
            .count()
            == 1,
        "the only crawl-delay belongs to a named agent, not to *"
    );
    let blocks: Vec<&str> = ROBOTS.split("User-agent:").collect();
    assert!(
        blocks
            .iter()
            .filter(|block| block.starts_with(" *"))
            .all(|block| !block.contains("Crawl-delay:")),
        "the wildcard agent block carries no crawl-delay"
    );
}

#[test]
fn wanted_names_normalise_the_requested_schools() {
    let wanted = wanted_names(&Options {
        school_names: vec!["  Cherry   Creek ".to_string(), "".to_string()],
        ..Options::default()
    });
    let names: HashSet<String> = wanted.into_iter().collect();
    assert!(names.contains(&census_domain::model::normalize_name("Cherry Creek")));
    assert_eq!(names.len(), 1);
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
        "fetched_at": "2026-09-27T12:00:00Z",
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
    let slug = cherry_creek_slug();
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    seed_cache(&cache, DIRECTORY_URL, DIRECTORY);
    seed_cache(&cache, &school_page_url(&slug), SCHOOL_PAGE);

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
        limit: None,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: vec![UsJurisdiction::Colorado],
        school_names: vec!["Cherry Creek".to_string()],
    };
    let report = collect(&ctx, &options)
        .await
        .expect("collect returns a report");

    assert_eq!(report.rows, 1, "only the requested school is fetched");
    assert_eq!(report.errors, 0);
    assert_eq!(report.with_email, 0, "CHSAA publishes no coach addresses");
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("50 coach row(s)")),
        "the report states the coach rows it wrote: {:?}",
        report.notes
    );
    assert_eq!(
        report.requests, 0,
        "both responses came from the seeded cache"
    );
    assert_eq!(
        report.from_cache, 2,
        "the directory page and the one school page"
    );

    let schools = store
        .scan::<CanonicalSchool>(census_store::Table::Schools)
        .expect("schools log");
    assert_eq!(schools.len(), 1);
    assert_eq!(schools[0].name, "Cherry Creek");
    assert_eq!(schools[0].state, Some(UsJurisdiction::Colorado));
    assert_eq!(schools[0].association.as_deref(), Some("CHSAA"));
    assert_eq!(schools[0].city.as_deref(), Some("Greenwood Village"));

    let coaches = store
        .scan::<CanonicalCoach>(census_store::Table::Coaches)
        .expect("coach log");
    assert_eq!(coaches.len(), 50);
    assert!(coaches
        .iter()
        .all(|coach| coach.school.as_str() == schools[0].id.as_str()));
    assert!(store
        .journal_keys("chsaa_schools")
        .expect("journal")
        .contains(format!("CO:{slug}").as_str()));
}

#[tokio::test]
async fn a_journalled_school_is_skipped_on_the_next_run() {
    let slug = cherry_creek_slug();
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    seed_cache(&cache, DIRECTORY_URL, DIRECTORY);
    seed_cache(&cache, &school_page_url(&slug), SCHOOL_PAGE);
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
        limit: None,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: vec![UsJurisdiction::Colorado],
        school_names: vec!["Cherry Creek".to_string()],
    };
    collect(&ctx, &options).await.expect("first run");
    let second = collect(&ctx, &options).await.expect("second run");
    assert_eq!(second.rows, 0, "the school was already journalled");
    assert_eq!(second.requests, 0, "a skipped school is not fetched again");
    assert!(
        second
            .notes
            .iter()
            .any(|note| note.contains("already journalled")),
        "the report says why nothing was processed: {:?}",
        second.notes
    );
}
