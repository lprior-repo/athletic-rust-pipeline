use super::collect::{requested_details, requested_letters};
use super::map::{map_contact_row, map_directory_row};
use super::parse::{parse_details, parse_directory};
use super::*;
use crate::AdapterReport;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender};
use census_domain::UsJurisdiction;
use serde_json::Value;
use std::collections::HashSet;

const DIRECTORY_A: &str = include_str!("../../tests/fixtures/pa_piaa/directory_alpha_a.html");
const DIRECTORY_B: &str = include_str!("../../tests/fixtures/pa_piaa/directory_alpha_b.html");
const DIRECTORY_Z: &str = include_str!("../../tests/fixtures/pa_piaa/directory_alpha_z.html");
const DETAILS_12048: &str = include_str!("../../tests/fixtures/pa_piaa/details_12048.html");
const ROBOTS: &str = include_str!("../../tests/fixtures/pa_piaa/robots.txt");
const GOLDEN_A: &str = include_str!("../../tests/fixtures/pa_piaa/golden_directory_alpha_a.json");
const GOLDEN_B: &str = include_str!("../../tests/fixtures/pa_piaa/golden_directory_alpha_b.json");
const GOLDEN_DETAILS: &str = include_str!("../../tests/fixtures/pa_piaa/golden_details_12048.json");

const OBSERVED_ON: &str = "2026-09-27";
const LIST_URL: &str = "https://www.piaa.org/schools/directory/list.aspx?alpha=A";

fn golden(json: &str) -> Value {
    serde_json::from_str(json).expect("the golden is the prototype's own JSON")
}

fn golden_schools(json: &str) -> Vec<Value> {
    golden(json)
        .get("schools")
        .and_then(Value::as_array)
        .cloned()
        .expect("the golden carries the prototype's schools")
}

fn golden_coaches(json: &str) -> Vec<Value> {
    golden(json)
        .get("coaches")
        .and_then(Value::as_array)
        .cloned()
        .expect("the golden carries the prototype's coaches")
}

fn field(row: &Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn compare_with_golden(body: &str, golden_json: &str) {
    let schools = parse_directory(body).expect("letter page parses");
    let expected = golden_schools(golden_json);
    assert_eq!(schools.len(), expected.len());
    for (rust, prototype) in schools.iter().zip(&expected) {
        assert_eq!(rust.name, field(prototype, "name").unwrap_or_default());
        assert_eq!(rust.street, field(prototype, "address"));
        assert_eq!(rust.city, field(prototype, "city"));
        assert_eq!(rust.state, field(prototype, "state"));
        assert_eq!(rust.zip, field(prototype, "zip"));
        assert_eq!(
            Some(rust.school_id.as_str()),
            field(prototype, "association_id").as_deref(),
            "the prototype's string association_id is the Rust row's school id"
        );
        assert_eq!(
            rust.detail_url,
            field(prototype, "detail_url").unwrap_or_default(),
            "the id names the prototype's own details URL"
        );
    }
}

#[test]
fn directory_letter_pages_match_the_prototype_golden_field_for_field() {
    compare_with_golden(DIRECTORY_A, GOLDEN_A);
    compare_with_golden(DIRECTORY_B, GOLDEN_B);
}

#[test]
fn the_directory_is_the_letters_the_site_links() {
    assert_eq!(LETTERS.len(), 24, "A..W plus Y");
    assert_eq!(LETTERS.first(), Some(&'A'));
    assert_eq!(LETTERS.last(), Some(&'Y'));
    assert!(!LETTERS.contains(&'X'), "X prints no link");
    assert!(!LETTERS.contains(&'Z'), "Z echoes the A group");
    assert_eq!(
        list_url('A'),
        "https://www.piaa.org/schools/directory/list.aspx?alpha=A"
    );
    assert_eq!(
        details_url("12048"),
        "https://www.piaa.org/schools/directory/details.aspx?ID=12048"
    );
}

#[test]
fn the_z_letter_page_echoes_the_a_group() {
    let a = parse_directory(DIRECTORY_A).expect("A parses");
    let z = parse_directory(DIRECTORY_Z).expect("Z parses");
    let names = |rows: &[super::parse::SchoolRow]| {
        rows.iter()
            .map(|row| row.name.clone())
            .collect::<Vec<String>>()
    };
    assert_eq!(
        names(&z),
        names(&a),
        "the site answers alpha=Z with the A group, which is why the crawl walks the linked letters only"
    );
}

#[test]
fn details_pages_keep_the_athletic_director_and_no_other_post() {
    assert!(
        DETAILS_12048.contains("Superintendent") && DETAILS_12048.contains("Principal"),
        "the capture publishes posts this adapter must not emit"
    );
    let page = parse_details(DETAILS_12048).expect("the details page parses");
    let expected = golden_coaches(GOLDEN_DETAILS);
    assert_eq!(
        golden_schools(GOLDEN_DETAILS)
            .first()
            .and_then(|row| field(row, "name")),
        Some(page.school_name.clone())
    );
    assert_eq!(page.contacts.len(), expected.len());
    for (rust, prototype) in page.contacts.iter().zip(&expected) {
        assert_eq!(rust.person, field(prototype, "person").unwrap_or_default());
        assert_eq!(rust.email, field(prototype, "email"));
    }
    assert_eq!(
        field(&expected[0], "role").as_deref(),
        Some("AthleticDirector")
    );
}

#[test]
fn an_entity_in_a_school_name_is_decoded_like_the_prototype() {
    let page = r#"<dl id="999" class="schoolBlock odd"><dt>A &amp; B School</dt>
        <dd>1 MAIN ST, PHILADELPHIA, PA  19103</dd></dl>"#;
    let rows = parse_directory(page).expect("the row parses");
    assert_eq!(rows[0].name, "A & B School");
    assert_eq!(rows[0].street.as_deref(), Some("1 MAIN ST"));
    assert_eq!(rows[0].city.as_deref(), Some("PHILADELPHIA"));
    assert_eq!(rows[0].zip.as_deref(), Some("19103"));
}

#[test]
fn malformed_pages_error_instead_of_panicking() {
    assert!(parse_directory("").is_err());
    assert!(parse_directory("<html><body>nothing here</body></html>").is_err());
    assert!(parse_details("").is_err());
    assert!(parse_details("<html><head><title></title></head></html>").is_err());
}

#[test]
fn robots_allows_the_directory_paths_this_adapter_reads() {
    assert!(
        !ROBOTS
            .lines()
            .filter(|line| line.starts_with("Disallow:"))
            .any(|line| line.contains("/schools")),
        "the member directory is open to the wildcard agent"
    );
    assert!(
        ROBOTS
            .lines()
            .any(|line| line.trim() == "Disallow: /officials/directory/"),
        "the officials directory is disallowed, which is why no administrator but the athletic director reaches this adapter"
    );
    assert!(
        !ROBOTS.to_lowercase().contains("crawl-delay"),
        "the host publishes no crawl delay; the adapter's 1 request/s is this project's own pace"
    );
}

#[test]
fn requested_letters_and_details_normalise_what_the_caller_names() {
    assert_eq!(requested_letters(&Options::default()), LETTERS.to_vec());
    assert_eq!(
        requested_letters(&Options {
            letters: vec!['A'],
            ..Options::default()
        }),
        vec!['A']
    );
    let details: HashSet<String> = requested_details(&Options {
        details_names: vec!["  A J McMullen   School ".to_string(), "".to_string()],
        ..Options::default()
    });
    assert!(details.contains(&census_domain::model::normalize_name("A J McMullen School")));
    assert_eq!(details.len(), 1);
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

fn options() -> Options {
    Options {
        limit: None,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: vec![UsJurisdiction::Pennsylvania],
        letters: vec!['A', 'B'],
        details_names: vec!["A J McMullen School".to_string()],
    }
}

async fn collect_from_cache(
    cache: &std::path::Path,
    store_dir: &std::path::Path,
) -> (AdapterReport, census_store::Store) {
    let store = census_store::Store::open(store_dir).expect("store");
    let fetcher = crate::net::Fetcher::new(
        cache,
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
    let report = collect(&ctx, &options())
        .await
        .expect("collect returns a report");
    (report, store)
}

#[tokio::test]
async fn collect_stores_the_letter_schools_and_the_requested_details_page_from_the_cache() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    seed_cache(&cache, &list_url('A'), DIRECTORY_A);
    seed_cache(&cache, &list_url('B'), DIRECTORY_B);
    seed_cache(&cache, &details_url("12048"), DETAILS_12048);

    let (report, store) = collect_from_cache(&cache, &dir.path().join("store")).await;

    assert_eq!(report.rows, 154, "53 schools on A plus 101 on B");
    assert_eq!(report.errors, 0);
    assert_eq!(
        report.requests, 0,
        "all three responses came from the seeded cache"
    );
    assert_eq!(
        report.from_cache, 3,
        "two letter pages and one details page"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("154 school(s) processed")),
        "the report states the schools it wrote: {:?}",
        report.notes
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("1 athletic-director row(s)")),
        "the report states the administrator rows it wrote: {:?}",
        report.notes
    );

    let schools = store
        .scan::<CanonicalSchool>(census_store::Table::Schools)
        .expect("schools log");
    assert_eq!(schools.len(), 154);
    let mcmullen = schools
        .iter()
        .find(|school| school.name == "A J McMullen School")
        .expect("the A page names A J McMullen School");
    assert_eq!(mcmullen.state, Some(UsJurisdiction::Pennsylvania));
    assert_eq!(mcmullen.association.as_deref(), Some("PIAA"));
    assert_eq!(mcmullen.city.as_deref(), Some("MARKLEYSBURG"));

    let coaches = store
        .scan::<CanonicalCoach>(census_store::Table::Coaches)
        .expect("coach log");
    assert_eq!(coaches.len(), 1, "only the requested details page was read");
    let director = &coaches[0];
    assert_eq!(director.school.as_str(), mcmullen.id.as_str());
    assert_eq!(director.role, CoachRole::AthleticDirector);
    assert_eq!(director.gender, Gender::Mixed);
    assert_eq!(director.sport, None, "an administrator post names no sport");
    assert!(
        director.has_published_email(),
        "the details page publishes it"
    );

    let journal = store.journal_keys("pa_piaa_schools").expect("journal");
    for key in ["PA:list:A", "PA:list:B", "PA:school:12048"] {
        assert!(journal.contains(key), "{key} is journalled");
    }
    assert_eq!(
        journal
            .iter()
            .filter(|key| key.starts_with("PA:school:"))
            .count(),
        154,
        "every listed school is journalled; only the requested one had a details page read, which is why the store holds one administrator row and the run spent three cache reads"
    );
}

#[tokio::test]
async fn a_journalled_letter_and_school_are_skipped_on_the_next_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    seed_cache(&cache, &list_url('A'), DIRECTORY_A);
    seed_cache(&cache, &list_url('B'), DIRECTORY_B);
    seed_cache(&cache, &details_url("12048"), DETAILS_12048);
    let store_dir = dir.path().join("store");
    let (first, store) = collect_from_cache(&cache, &store_dir).await;
    assert_eq!(first.rows, 154);
    drop(store);
    let (second, _) = collect_from_cache(&cache, &store_dir).await;
    assert_eq!(second.rows, 0, "both letters were already journalled");
    assert_eq!(second.requests, 0, "a skipped letter is not fetched again");
    assert!(
        second
            .notes
            .iter()
            .any(|note| note.contains("already journalled")),
        "the report says why nothing was processed: {:?}",
        second.notes
    );
}

#[test]
fn mapped_rows_carry_the_source_identity_and_evidence() {
    let rows = parse_directory(DIRECTORY_A).expect("A parses");
    let (school, id) = map_directory_row(&rows[0], LIST_URL, OBSERVED_ON).expect("a school");
    assert_eq!(school.name, "A J McMullen School");
    assert_eq!(school.state, Some(UsJurisdiction::Pennsylvania));
    assert_eq!(school.association.as_deref(), Some("PIAA"));
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(
        school.source_identities[0].url.as_deref(),
        Some("https://www.piaa.org/schools/directory/details.aspx?ID=12048")
    );
    let page = parse_details(DETAILS_12048).expect("the details page parses");
    let coach = map_contact_row(&page.contacts[0], &id, &details_url("12048"), OBSERVED_ON)
        .expect("an administrator row");
    assert_eq!(coach.role, CoachRole::AthleticDirector);
    assert!(coach.has_published_email());
}
