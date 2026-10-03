use super::collect::wanted_names;
use super::map::{map_coach_row, map_directory_row};
use super::parse::{parse_directory, parse_school_page};
use super::*;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DIRECTORY: &str = include_str!("../../tests/fixtures/chsaa/directory.html");
const SCHOOL_PAGE: &str = include_str!("../../tests/fixtures/chsaa/school_cherry_creek.html");
const ROBOTS: &str = include_str!("../../tests/fixtures/chsaa/robots.txt");
const GOLDEN_DIRECTORY: &str =
    include_str!("../../tests/fixtures/chsaa/golden_directory_rows.json");
const GOLDEN_COACHES: &str =
    include_str!("../../tests/fixtures/chsaa/golden_school_coach_rows.json");

const OBSERVED_ON: &str = "2026-09-27";

fn golden_rows(json: &str) -> TestResult<Vec<Value>> {
    Ok(serde_json::from_str(json)?)
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
        coach
            .sport
            .map(sport_key)
            .map_or("none", |value| value)
            .to_string(),
        format!("{:?}", coach.role),
        format!("{:?}", coach.gender),
    )
}

fn cherry_creek() -> TestResult<super::parse::MemberSchool> {
    let schools = parse_directory(DIRECTORY)?;
    Ok(schools
        .into_iter()
        .find(|school| school.name.as_deref() == Some("Cherry Creek"))
        .ok_or("the directory holds Cherry Creek")?)
}

fn cherry_creek_slug() -> TestResult<String> {
    Ok(cherry_creek()?
        .slug
        .ok_or("Cherry Creek rows carry a slug")?)
}

#[test]
fn directory_parses_the_378_member_schools() -> TestResult {
    let schools = parse_directory(DIRECTORY)?;
    check!(eq; schools.len(), 378, "the capture holds 378 member schools");
    check!(eq; schools[0].name.as_deref(), Some("Academy"));
    Ok(())
}

#[test]
fn directory_rows_match_the_prototype_golden_field_for_field() -> TestResult {
    let schools = parse_directory(DIRECTORY)?;
    let golden = golden_rows(GOLDEN_DIRECTORY)?;
    check!(eq; schools.len(), golden.len());
    for (rust, prototype) in schools.iter().zip(&golden) {
        check!(eq; rust.name.as_deref(), field(prototype, "name").as_deref());
        check!(eq;
            rust.official_name.as_deref(),
            field(prototype, "official_name").as_deref()
        );
        check!(eq; rust.city.as_deref(), field(prototype, "city").as_deref());
        check!(eq;
            rust.street_address.as_deref(),
            field(prototype, "address").as_deref()
        );
        check!(eq; rust.zip_code.as_deref(), field(prototype, "zip").as_deref());
        check!(eq; rust.phone.as_deref(), field(prototype, "phone").as_deref());
        check!(eq;
            rust.district_name.as_deref(),
            field(prototype, "district").as_deref()
        );
        check!(eq;
            rust.member_type.as_deref(),
            field(prototype, "member_type").as_deref()
        );
        check!(eq;
            rust.school_type.as_deref(),
            field(prototype, "school_type").as_deref()
        );
        check!(eq;
            rust.setting.as_deref(),
            field(prototype, "setting").as_deref()
        );
        check!(eq;
            rust.school_code.map(|code| code.to_string()).as_deref(),
            field(prototype, "association_id").as_deref(),
            "the prototype's string association_id is the Rust row's numeric school_code"
        );
        let slug = rust.slug.as_deref().ok_or("every golden row has a slug")?;
        check!(eq;
            school_page_url(slug),
            field(prototype, "detail_url").map_or(Default::default(), core::convert::identity),
            "the slug names the prototype's own school URL"
        );
    }
    Ok(())
}

#[test]
fn directory_address_coverage_matches_the_recorded_figures() -> TestResult {
    let schools = parse_directory(DIRECTORY)?;
    let with_address = schools
        .iter()
        .filter(|school| school.street_address.is_some())
        .count();
    let with_zip = schools
        .iter()
        .filter(|school| school.zip_code.is_some())
        .count();
    check!(eq;
        with_address, 378,
        "the applicability record cites 378/378 addresses"
    );
    check!(eq; with_zip, 376, "the applicability record cites 376/378 zips");
    Ok(())
}

#[test]
fn school_page_rows_match_the_prototype_golden_after_mapping() -> TestResult {
    let member = cherry_creek()?;
    let (_, school_id) =
        map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).ok_or("Cherry Creek is a school")?;
    let url = school_page_url(
        member
            .slug
            .as_deref()
            .map_or(Default::default(), core::convert::identity),
    );
    let rows = parse_school_page(SCHOOL_PAGE)?;
    let mut rust: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    for row in &rows {
        if let Some(coach) = map_coach_row(row, &school_id, &url, OBSERVED_ON) {
            *rust.entry(coach_key(&coach)).or_insert(0) += 1;
        }
    }
    let prototype = golden_rows(GOLDEN_COACHES)?;
    let mut expected: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    for row in &prototype {
        if field(row, "sport").as_deref() == Some("Track")
            || field(row, "sport").as_deref() == Some("CrossCountry")
        {
            let key = (
                field(row, "person").map_or(Default::default(), core::convert::identity),
                field(row, "sport").map_or(Default::default(), core::convert::identity),
                field(row, "role").map_or(Default::default(), core::convert::identity),
                field(row, "gender").map_or(Default::default(), core::convert::identity),
            );
            *expected.entry(key).or_insert(0) += 1;
        }
    }
    check!(eq;
        rust, expected,
        "the Rust (person, sport, role, gender) set equals the prototype's; the prototype's level column is uniform Varsity here and the census model stores no coach level"
    );
    check!(
        rows.len() > 50,
        "the page lists every activity it publishes; the mapper is what narrows to track and cross country"
    );
    check!(eq; rust.values().sum::<usize>(), 50);
    Ok(())
}

#[test]
fn school_page_names_the_school_from_its_title() -> TestResult {
    let rows = parse_school_page(SCHOOL_PAGE)?;
    check!(!rows.is_empty());
    check!(rows.iter().all(|row| row.school_name == "Cherry Creek"));
    Ok(())
}

#[test]
fn mapped_coaches_are_never_athletic_directors() -> TestResult {
    let member = cherry_creek()?;
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).ok_or("school")?;
    for row in parse_school_page(SCHOOL_PAGE)? {
        if let Some(coach) = map_coach_row(&row, &school_id, DIRECTORY_URL, OBSERVED_ON) {
            check!(ne; coach.role, CoachRole::AthleticDirector);
        }
    }
    Ok(())
}

#[test]
fn sports_outside_track_and_cross_country_are_dropped() -> TestResult {
    let member = cherry_creek()?;
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).ok_or("school")?;
    let row = super::parse::SchoolCoachRow {
        school_name: "Cherry Creek".to_string(),
        person: "Someone".to_string(),
        activity_name: "Boys Basketball".to_string(),
        title: "Head Coach".to_string(),
    };
    check!(map_coach_row(&row, &school_id, DIRECTORY_URL, OBSERVED_ON).is_none());
    Ok(())
}

#[test]
fn a_role_outside_the_coaching_titles_is_dropped() -> TestResult {
    let member = cherry_creek()?;
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).ok_or("school")?;
    let row = super::parse::SchoolCoachRow {
        school_name: "Cherry Creek".to_string(),
        person: "Someone".to_string(),
        activity_name: "Boys Cross Country".to_string(),
        title: "Volunteer".to_string(),
    };
    check!(map_coach_row(&row, &school_id, DIRECTORY_URL, OBSERVED_ON).is_none());
    Ok(())
}

#[test]
fn gender_comes_from_the_activity_name() -> TestResult {
    let member = cherry_creek()?;
    let (_, school_id) = map_directory_row(&member, DIRECTORY_URL, OBSERVED_ON).ok_or("school")?;
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
    check!(eq;
        map_coach_row(&boys, &school_id, DIRECTORY_URL, OBSERVED_ON).map(|coach| coach.gender),
        Some(Gender::Boys)
    );
    let coach = map_coach_row(&girls, &school_id, DIRECTORY_URL, OBSERVED_ON).ok_or("girls row")?;
    check!(eq; coach.gender, Gender::Girls);
    check!(eq; coach.sport, Some(Sport::OutdoorTrack));
    Ok(())
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
fn descriptor_is_registered_with_the_directory_capabilities() -> TestResult {
    let entry = crate::registry::descriptor("chsaa").ok_or("chsaa is registered")?;
    check!(eq; entry.transport, crate::registry::TransportKind::Html);
    check!(entry.capabilities.school_evidence);
    check!(entry.capabilities.coach_directory);
    check!(!entry.capabilities.public_professional_contact);
    Ok(())
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
        "fetched_at": "2026-09-27T12:00:00Z",
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
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let slug = cherry_creek_slug()?;
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            seed_cache(&cache, DIRECTORY_URL, DIRECTORY)?;
            seed_cache(&cache, &school_page_url(&slug), SCHOOL_PAGE)?;

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
                school_year: census_domain::model::SchoolYear::new(2026)
                    .ok_or("2026 is a season")?,
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
            let report = collect(&ctx, &options).await?;

            check!(eq; report.rows, 1, "only the requested school is fetched");
            check!(eq; report.errors, 0);
            check!(eq; report.with_email, 0, "CHSAA publishes no coach addresses");
            check!(
                report
                    .notes
                    .iter()
                    .any(|note| note.contains("50 coach row(s)")),
                "the report states the coach rows it wrote: {:?}",
                report.notes
            );
            check!(eq;
                report.requests, 0,
                "both responses came from the seeded cache"
            );
            check!(eq;
                report.from_cache, 2,
                "the directory page and the one school page"
            );

            let schools = store.scan::<CanonicalSchool>(census_store::Table::Schools)?;
            check!(eq; schools.len(), 1);
            check!(eq; schools[0].name, "Cherry Creek");
            check!(eq; schools[0].state, Some(UsJurisdiction::Colorado));
            check!(eq; schools[0].association.as_deref(), Some("CHSAA"));
            check!(eq; schools[0].city.as_deref(), Some("Greenwood Village"));

            let coaches = store.scan::<CanonicalCoach>(census_store::Table::Coaches)?;
            check!(eq; coaches.len(), 50);
            check!(coaches
                .iter()
                .all(|coach| coach.school.as_str() == schools[0].id.as_str()));
            check!(store
                .journal_keys("chsaa_schools")?
                .contains(format!("CO:{slug}").as_str()));
            Ok(())
        })
}

#[test]
fn a_journalled_school_is_skipped_on_the_next_run() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let slug = cherry_creek_slug()?;
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            seed_cache(&cache, DIRECTORY_URL, DIRECTORY)?;
            seed_cache(&cache, &school_page_url(&slug), SCHOOL_PAGE)?;
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
                school_year: census_domain::model::SchoolYear::new(2026)
                    .ok_or("2026 is a season")?,
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
            collect(&ctx, &options).await?;
            let second = collect(&ctx, &options).await?;
            check!(eq; second.rows, 0, "the school was already journalled");
            check!(eq; second.requests, 0, "a skipped school is not fetched again");
            check!(
                second
                    .notes
                    .iter()
                    .any(|note| note.contains("already journalled")),
                "the report says why nothing was processed: {:?}",
                second.notes
            );
            Ok(())
        })
}
