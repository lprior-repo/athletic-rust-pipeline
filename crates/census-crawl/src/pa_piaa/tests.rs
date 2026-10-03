use super::collect::requested_details;
use super::map::{map_contact_row, map_directory_row};
use super::parse::{parse_details, parse_directory};
use super::*;
use crate::AdapterReport;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender};
use census_domain::UsJurisdiction;
use serde_json::Value;
use std::collections::HashSet;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DIRECTORY_A: &str = include_str!("../../tests/fixtures/pa_piaa/directory_alpha_a.html");
const DIRECTORY_B: &str = include_str!("../../tests/fixtures/pa_piaa/directory_alpha_b.html");
const DIRECTORY_Z: &str = include_str!("../../tests/fixtures/pa_piaa/directory_alpha_z.html");
const DETAILS_12048: &str = include_str!("../../tests/fixtures/pa_piaa/details_12048.html");
const GOLDEN_A: &str = include_str!("../../tests/fixtures/pa_piaa/golden_directory_alpha_a.json");
const GOLDEN_B: &str = include_str!("../../tests/fixtures/pa_piaa/golden_directory_alpha_b.json");
const GOLDEN_DETAILS: &str = include_str!("../../tests/fixtures/pa_piaa/golden_details_12048.json");

const OBSERVED_ON: &str = "2026-09-27";
const LIST_URL: &str = "https://www.piaa.org/schools/directory/list.aspx?alpha=A";

fn golden(json: &str) -> TestResult<Value> {
    Ok(serde_json::from_str(json)?)
}

fn golden_schools(json: &str) -> TestResult<Vec<Value>> {
    Ok(golden(json)?
        .get("schools")
        .and_then(Value::as_array)
        .cloned()
        .ok_or("the golden carries the prototype's schools")?)
}

fn golden_coaches(json: &str) -> TestResult<Vec<Value>> {
    Ok(golden(json)?
        .get("coaches")
        .and_then(Value::as_array)
        .cloned()
        .ok_or("the golden carries the prototype's coaches")?)
}

fn field(row: &Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn compare_with_golden(body: &str, golden_json: &str) -> TestResult {
    let schools = parse_directory(body)?;
    let expected = golden_schools(golden_json)?;
    check!(eq; schools.len(), expected.len());
    for (rust, prototype) in schools.iter().zip(&expected) {
        check!(eq; rust.name, field(prototype, "name").map_or(Default::default(), core::convert::identity));
        check!(eq; rust.street, field(prototype, "address"));
        check!(eq; rust.city, field(prototype, "city"));
        check!(eq; rust.state, field(prototype, "state"));
        check!(eq; rust.zip, field(prototype, "zip"));
        check!(eq;
            Some(rust.school_id.as_str()),
            field(prototype, "association_id").as_deref(),
            "the prototype's string association_id is the Rust row's school id"
        );
        check!(eq;
            rust.detail_url,
            field(prototype, "detail_url").map_or(Default::default(), core::convert::identity),
            "the id names the prototype's own details URL"
        );
    }
    Ok(())
}

#[test]
fn directory_letter_pages_match_the_prototype_golden_field_for_field() -> TestResult {
    compare_with_golden(DIRECTORY_A, GOLDEN_A)?;
    compare_with_golden(DIRECTORY_B, GOLDEN_B)?;
    Ok(())
}

#[test]
fn the_z_letter_page_echoes_the_a_group() -> TestResult {
    let a = parse_directory(DIRECTORY_A)?;
    let z = parse_directory(DIRECTORY_Z)?;
    let names = |rows: &[super::parse::SchoolRow]| {
        rows.iter()
            .map(|row| row.name.clone())
            .collect::<Vec<String>>()
    };
    check!(eq;
        names(&z),
        names(&a),
        "the site answers alpha=Z with the A group, which is why the crawl walks the linked letters only"
    );
    Ok(())
}

#[test]
fn details_pages_keep_the_athletic_director_and_no_other_post() -> TestResult {
    let page = parse_details(DETAILS_12048)?;
    let expected = golden_coaches(GOLDEN_DETAILS)?;
    check!(eq;
        golden_schools(GOLDEN_DETAILS)?
            .first()
            .and_then(|row| field(row, "name")),
        Some(page.school_name.clone())
    );
    check!(eq; page.contacts.len(), expected.len());
    for (rust, prototype) in page.contacts.iter().zip(&expected) {
        check!(eq; rust.person, field(prototype, "person").map_or(Default::default(), core::convert::identity));
        check!(eq; rust.email, field(prototype, "email"));
    }
    Ok(())
}

#[test]
fn an_entity_in_a_school_name_is_decoded_like_the_prototype() -> TestResult {
    let page = r#"<dl id="999" class="schoolBlock odd"><dt>A &amp; B School</dt>
        <dd>1 MAIN ST, PHILADELPHIA, PA  19103</dd></dl>"#;
    let rows = parse_directory(page)?;
    check!(eq; rows[0].name, "A & B School");
    check!(eq; rows[0].street.as_deref(), Some("1 MAIN ST"));
    check!(eq; rows[0].city.as_deref(), Some("PHILADELPHIA"));
    check!(eq; rows[0].zip.as_deref(), Some("19103"));
    Ok(())
}

#[test]
fn malformed_pages_error_instead_of_panicking() {
    assert!(parse_directory("").is_err());
    assert!(parse_directory("<html><body>nothing here</body></html>").is_err());
    assert!(parse_details("").is_err());
    assert!(parse_details("<html><head><title></title></head></html>").is_err());
}

#[test]
fn detail_name_filter_normalises_whitespace_and_discards_empty_names() {
    let details: HashSet<String> = requested_details(&Options {
        details_names: vec!["  A J McMullen   School ".to_string(), "".to_string()],
        ..Options::default()
    });
    assert!(details.contains(&census_domain::model::normalize_name("A J McMullen School")));
    assert_eq!(details.len(), 1);
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
) -> TestResult<(AdapterReport, census_store::Store)> {
    let store = census_store::Store::open(store_dir)?;
    let fetcher = crate::net::Fetcher::new(
        cache,
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    let ctx = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("2026 is a season")?,
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    };
    let report = collect(&ctx, &options()).await?;
    Ok((report, store))
}

#[test]
fn collect_stores_the_letter_schools_and_the_requested_details_page_from_the_cache() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    seed_cache(&cache, &list_url('A'), DIRECTORY_A)?;
    seed_cache(&cache, &list_url('B'), DIRECTORY_B)?;
    seed_cache(&cache, &details_url("12048"), DETAILS_12048)?;

    let (report, store) = collect_from_cache(&cache, &dir.path().join("store")).await?;

    check!(eq; report.rows, 154, "53 schools on A plus 101 on B");
    check!(eq; report.errors, 0);
    check!(eq;
        report.requests, 0,
        "all three responses came from the seeded cache"
    );
    check!(eq;
        report.from_cache, 3,
        "two letter pages and one details page"
    );

    let schools = store
        .scan::<CanonicalSchool>(census_store::Table::Schools)
        ?;
    check!(eq; schools.len(), 154);
    let mcmullen = schools
        .iter()
        .find(|school| school.name == "A J McMullen School")
        .ok_or("the A page names A J McMullen School")?;
    check!(eq; mcmullen.state, Some(UsJurisdiction::Pennsylvania));
    check!(eq; mcmullen.association.as_deref(), Some("PIAA"));
    check!(eq; mcmullen.city.as_deref(), Some("MARKLEYSBURG"));

    let coaches = store
        .scan::<CanonicalCoach>(census_store::Table::Coaches)
        ?;
    check!(eq; coaches.len(), 1, "only the requested details page was read");
    let director = &coaches[0];
    check!(eq; director.school.as_str(), mcmullen.id.as_str());
    check!(eq; director.role, CoachRole::AthleticDirector);
    check!(eq; director.gender, Gender::Mixed);
    check!(eq; director.sport, None, "an administrator post names no sport");
    check!(
        director.has_published_email(),
        "the details page publishes it"
    );

    let journal = store.journal_keys("pa_piaa_schools")?;
    for key in ["PA:list:A", "PA:list:B", "PA:school:12048"] {
        check!(journal.contains(key), "{key} is journalled");
    }
    check!(eq;
        journal
            .iter()
            .filter(|key| key.starts_with("PA:school:"))
            .count(),
        154,
        "every listed school is journalled; only the requested one had a details page read, which is why the store holds one administrator row and the run spent three cache reads"
    );
    Ok(())
    })
}

#[test]
fn a_journalled_letter_and_school_are_skipped_on_the_next_run() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            seed_cache(&cache, &list_url('A'), DIRECTORY_A)?;
            seed_cache(&cache, &list_url('B'), DIRECTORY_B)?;
            seed_cache(&cache, &details_url("12048"), DETAILS_12048)?;
            let store_dir = dir.path().join("store");
            let (first, store) = collect_from_cache(&cache, &store_dir).await?;
            check!(eq; first.rows, 154);
            drop(store);
            let (second, _) = collect_from_cache(&cache, &store_dir).await?;
            check!(eq; second.rows, 0, "both letters were already journalled");
            check!(eq; second.requests, 0, "a skipped letter is not fetched again");
            Ok(())
        })
}

#[test]
fn mapped_rows_carry_the_source_identity_and_evidence() -> TestResult {
    let rows = parse_directory(DIRECTORY_A)?;
    let (school, id) = map_directory_row(&rows[0], LIST_URL, OBSERVED_ON).ok_or("a school")?;
    check!(eq; school.name, "A J McMullen School");
    check!(eq; school.state, Some(UsJurisdiction::Pennsylvania));
    check!(eq; school.association.as_deref(), Some("PIAA"));
    check!(eq; school.source_identities.len(), 1);
    check!(eq;
        school.source_identities[0].url.as_deref(),
        Some("https://www.piaa.org/schools/directory/details.aspx?ID=12048")
    );
    let page = parse_details(DETAILS_12048)?;
    let coach = map_contact_row(&page.contacts[0], &id, &details_url("12048"), OBSERVED_ON)
        .ok_or("an administrator row")?;
    check!(eq; coach.role, CoachRole::AthleticDirector);
    check!(coach.has_published_email());
    Ok(())
}
