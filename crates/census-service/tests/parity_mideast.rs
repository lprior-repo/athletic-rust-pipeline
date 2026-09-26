//! Golden-corpus parity harness for the mid-east association and timer adapters.
//!
//! A decomposition refactor moves code between functions and files; this harness is the proof that
//! no published parse result moves with it. Every fixture in each source's committed corpus is
//! parsed and compared byte-for-byte against a checked-in golden under `tests/golden/`, and the
//! per-case digests are rolled up into one `<source>__rollup` golden so a fixture that stops being
//! walked fails the run instead of dropping quietly out of coverage.
//!
//! Sources: `ohsaa` (Ohio), `ihsa` (Illinois), `ks` (Kansas), `wayzata` (the Minnesota timer's
//! schedules) and `compiled` (the WIAA "Compiled" result export).
//!
//! `compiled` has no fixture directory of its own: its `#[cfg(test)]` corpus is an inline export
//! literal, and every committed body under `tests/fixtures/wiaa_results/` that the compiled reader
//! claims is claimed first — and published — by the Hy-Tek reader (`wiaa_results::parse_pdf` tries
//! Hy-Tek, then compiled, then cross-country; `.txt` artifacts go straight to Hy-Tek). Its cases
//! below therefore carry the documented export text verbatim.
//!
//! Seed with `GOLDEN_UPDATE=1 cargo nextest run -p census-service --test parity_mideast`, inspect
//! the diff, and re-run without the variable: the run must be green and byte-stable.

mod common;

use census_domain::UsJurisdiction;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use census_crawl::compiled;
use census_crawl::hytek;
use census_crawl::net::Fetcher;
use census_crawl::wayzata::{self, ScheduleSport};
use census_crawl::{ihsa, ks, ohsaa, AdapterContext, AdapterReport};
use census_domain::model::{
    CanonicalCoach, CanonicalMeet, CanonicalSchool, CompetitionLevel, EventKind, Gender, Grade,
    Mark, SchoolId, SchoolYear, SourceRef, Sport,
};
use census_store::{Store, Table};
use serde::Serialize;

/// Evidence date stamped into every entity a seeded run mints. Fixed so the goldens are stable.
const OBSERVED_ON: &str = "2026-09-20";
/// Season the schedule fixtures belong to.
const SEASON: i16 = 2026;
/// Capture time of the seeded cache entries (the fetcher only reads it back).
const SEEDED_AT: &str = "2026-09-20T14:39:00Z";


/// The fixture file name without its extension.
fn stem_of(file: &str) -> &str {
    file.split_once('.').map_or(file, |(stem, _)| stem)
}

/// Seed the fetcher's on-disk cache with one body under the key the fetcher derives for `GET url`.
///
/// The key derivation (`sha256("GET" 0x1f url 0x1f)[..16]`, hex) is part of the cache layout, so
/// writing the two cache files directly is what makes a run hermetic: every request the adapter
/// makes below is answered from this seed and no test ever reaches the network.
fn seed_cache(cache_dir: &Path, url: &str, body: &str) -> Result<()> {
    use sha2::{Digest as _, Sha256};

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
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": SEEDED_AT,
    });

    let dir = cache_dir.to_path_buf();
    std::fs::write(
        dir.join(format!("{key}.meta.json")),
        serde_json::to_string(&meta).context("serializing the seeded cache metadata")?,
    )
    .with_context(|| format!("writing the cache metadata for {url}"))?;
    std::fs::write(dir.join(format!("{key}.body")), body)
        .with_context(|| format!("writing the cache body for {url}"))?;
    Ok(())
}

/// A run rooted in `dir`: a Fjall store plus a fetcher whose cache holds exactly `bodies`.
fn seeded(dir: &Path, bodies: &[(&str, &str)]) -> Result<(Store, Fetcher)> {
    let cache = dir.join("http");
    std::fs::create_dir_all(&cache).context("creating the seeded cache directory")?;
    for (url, body) in bodies {
        seed_cache(&cache, url, body)?;
    }
    let store = Store::open(dir.join("store")).context("opening the run's store")?;
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .context("building the fetcher")?;
    Ok((store, fetcher))
}

/// The per-run context every adapter call is handed.
fn context<'a>(fetcher: &'a Fetcher, store: &'a Store) -> AdapterContext<'a> {
    AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(SEASON).expect("SEASON is a season"),
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    }
}

/// Write one case's golden and return its `(case name, digest)` pair for the directory rollup.
fn golden_case<T: Serialize>(name: &str, value: &T) -> Result<(String, String)> {
    common::assert_golden(name, value)?;
    Ok((name.to_string(), common::digest(value)?))
}

/// A run's published outcome: what the adapter reported and every entity it appended, as the store
/// merges them back.
#[derive(Serialize)]
struct SchoolsRun<'a> {
    report: &'a AdapterReport,
    schools: &'a [CanonicalSchool],
    coaches: &'a [CanonicalCoach],
}

/// A timer run's published outcome.
#[derive(Serialize)]
struct MeetsRun<'a> {
    report: &'a AdapterReport,
    meets: &'a [CanonicalMeet],
}

/// One source's rollup: the per-case digests plus the number of inputs they came from — the fixture
/// files of a fixture-backed source, the documented export bodies of a source that has none.
#[derive(Serialize)]
struct Rollup {
    cases: BTreeMap<String, String>,
    inputs: usize,
}

/// Assert one directory's rollup against its golden, refusing a run that walked fewer inputs than
/// the corpus holds.
fn assert_rollup(source: &str, cases: BTreeMap<String, String>, inputs: usize) -> Result<()> {
    if cases.len() != inputs {
        bail!(
            "{source}: {} cases for {inputs} inputs — an input did not produce a case",
            cases.len()
        );
    }
    common::assert_golden(&format!("{source}__rollup"), &Rollup { cases, inputs })
}


/// One search-result row, with the page URLs its id derives.
#[derive(Serialize)]
struct SearchRow {
    name: String,
    city: String,
    ohsaa_id: String,
    page_url: String,
    sports_url: String,
    ad_url: String,
}

impl SearchRow {
    fn of(result: &ohsaa::SearchResult) -> Self {
        let ohsaa::SearchResult {
            name,
            city,
            ohsaa_id,
        } = result;
        Self {
            name: name.clone(),
            city: city.clone(),
            ohsaa_id: ohsaa_id.clone(),
            page_url: result.page_url(),
            sports_url: result.sports_url(),
            ad_url: result.ad_url(),
        }
    }
}

/// One coach cell of the sports-information table.
#[derive(Serialize)]
struct CoachCell {
    name: String,
    email: Option<String>,
}

impl CoachCell {
    fn of(entry: &ohsaa::CoachEntry) -> Self {
        let ohsaa::CoachEntry { name, email } = entry;
        Self {
            name: name.clone(),
            email: email.clone(),
        }
    }
}

/// One sports-information row, with the sport its label maps to.
#[derive(Serialize)]
struct SportsRow {
    label: String,
    sport: Option<Sport>,
    boys: Option<CoachCell>,
    girls: Option<CoachCell>,
}

impl SportsRow {
    fn of(section: &(String, Option<ohsaa::CoachEntry>, Option<ohsaa::CoachEntry>)) -> Self {
        let (label, boys, girls) = section;
        Self {
            label: label.clone(),
            sport: ohsaa::parse_sport_label(label),
            boys: boys.as_ref().map(CoachCell::of),
            girls: girls.as_ref().map(CoachCell::of),
        }
    }
}

/// The athletic-department table.
#[derive(Serialize)]
struct AdFacts {
    director: Option<(String, Option<String>)>,
    office_roles: Vec<(String, String)>,
}

impl AdFacts {
    fn of(page: &ohsaa::AdPage) -> Self {
        let ohsaa::AdPage {
            director,
            office_roles,
        } = page;
        Self {
            director: director.clone(),
            office_roles: office_roles.clone(),
        }
    }
}

/// Everything one search page publishes: its deduplicated rows, the name resolution the adapter
/// runs against them, and the notes that resolution produced.
#[derive(Serialize)]
struct SearchFacts {
    rows: Vec<SearchRow>,
    query: String,
    resolved: Option<SearchRow>,
    notes: Vec<String>,
}

/// The canonical entities one school's page set mints.
#[derive(Serialize)]
struct SchoolEntities<'a> {
    school: &'a CanonicalSchool,
    school_id: &'a SchoolId,
    coaches: &'a [CanonicalCoach],
}

/// Parse one OHSAA fixture as the page kind its name declares, write its golden, and return the
/// case name with its digest.
fn ohsaa_case(file: &str, body: &str) -> Result<(String, String)> {
    let stem = stem_of(file);
    let name = format!("ohsaa__{stem}");
    if stem.starts_with("search_") {
        let rows = ohsaa::parse_search(body);
        let query = rows.first().map_or(String::new(), |row| row.name.clone());
        let mut notes = Vec::new();
        let resolved = ohsaa::resolve_school_name(body, &query, &mut notes);
        let facts = SearchFacts {
            rows: rows.iter().map(SearchRow::of).collect(),
            query,
            resolved: resolved.as_ref().map(SearchRow::of),
            notes,
        };
        golden_case(&name, &facts)
    } else if stem.starts_with("sports_") {
        let sections = ohsaa::parse_sports_table(body);
        golden_case(
            &name,
            &sections.iter().map(SportsRow::of).collect::<Vec<_>>(),
        )
    } else if stem.starts_with("ad_") {
        golden_case(&name, &AdFacts::of(&ohsaa::parse_ad_page(body)))
    } else {
        bail!("fixture {file} matches no OHSAA page kind: search_, sports_ or ad_")
    }
}

/// Every committed OHSAA fixture, parsed by the page kind its name declares.
#[test]
fn ohsaa_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("ohsaa")?;
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let (name, digest) = ohsaa_case(&file, &common::fixture("ohsaa", &file)?)?;
        cases.insert(name, digest);
    }
    assert_rollup("ohsaa", cases, paths.len())
}

/// The entity-minting path: one school's search row plus its sports and AD pages become a canonical
/// school and its coach rows, ids included.
#[test]
fn ohsaa_coffman_entities_match_golden() -> Result<()> {
    let rows = ohsaa::parse_search(&common::fixture("ohsaa", "search_dublin_coffman.html")?);
    let result = rows
        .first()
        .context("the search fixture carries Dublin Coffman")?;
    let extract = ohsaa::school_entities(
        result,
        &common::fixture("ohsaa", "sports_dublin_coffman.html")?,
        &common::fixture("ohsaa", "ad_dublin_coffman.html")?,
        OBSERVED_ON,
    );
    let ohsaa::SchoolExtract {
        school,
        school_id,
        coaches,
    } = &extract;
    common::assert_golden(
        "ohsaa__coffman_entities",
        &SchoolEntities {
            school,
            school_id,
            coaches,
        },
    )
}

/// The whole adapter over a seeded cache: one requested school, its two pages, and the entities the
/// store ends up holding.
#[tokio::test]
async fn ohsaa_collect_from_a_seeded_cache_matches_golden() -> Result<()> {
    let rows = ohsaa::parse_search(&common::fixture("ohsaa", "search_dublin_coffman.html")?);
    let result = rows
        .first()
        .context("the search fixture carries Dublin Coffman")?;
    let search_url = format!("{}/Outside/SearchSchool?Name=Dublin%20Coffman", ohsaa::HOST);

    let dir = tempfile::tempdir().context("temp dir")?;
    let (store, fetcher) = seeded(
        dir.path(),
        &[
            (
                &search_url,
                &common::fixture("ohsaa", "search_dublin_coffman.html")?,
            ),
            (
                &result.sports_url(),
                &common::fixture("ohsaa", "sports_dublin_coffman.html")?,
            ),
            (
                &result.ad_url(),
                &common::fixture("ohsaa", "ad_dublin_coffman.html")?,
            ),
        ],
    )?;
    let options = ohsaa::Options {
        limit: None,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: vec![UsJurisdiction::Ohio],
        school_names: vec!["Dublin Coffman".to_string()],
    };
    let report = ohsaa::collect(&context(&fetcher, &store), &options).await?;

    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    anyhow::ensure!(
        report.requests == 0,
        "the seeded cache answers every request — left={:?} right={:?}",
        &report.requests,
        &0
    );
    anyhow::ensure!(
        report.from_cache == 3,
        "search, sports and AD pages are cached — left={:?} right={:?}",
        &report.from_cache,
        &3
    );
    {
        let left_value = &(schools.len(), coaches.len());
        let right_value = &(1, 5);
        anyhow::ensure!(left_value == right_value, "Dublin Coffman mints one school and its coach rows: {report:?} — left={left_value:?} right={right_value:?}");
    }
    common::assert_golden(
        "ohsaa__collect_coffman",
        &SchoolsRun {
            report: &report,
            schools: &schools,
            coaches: &coaches,
        },
    )
}


/// One row of the `/v1/schools` payload.
#[derive(Serialize)]
struct SchoolRow {
    school_id: String,
    name_formal: String,
    name_ihsa: Option<String>,
    name_short: Option<String>,
    city: String,
    membership_type: Option<String>,
    kind: Option<String>,
    enrollment_type: Option<String>,
    has_boundary: Option<String>,
    is_cps: Option<String>,
    url: Option<String>,
}

impl SchoolRow {
    fn of(record: &ihsa::SchoolRecord) -> Self {
        let ihsa::SchoolRecord {
            school_id,
            name_formal,
            name_ihsa,
            name_short,
            city,
            membership_type,
            r#type,
            enrollment_type,
            has_boundary,
            is_cps,
            url,
        } = record;
        Self {
            school_id: school_id.clone(),
            name_formal: name_formal.clone(),
            name_ihsa: name_ihsa.clone(),
            name_short: name_short.clone(),
            city: city.clone(),
            membership_type: membership_type.clone(),
            kind: r#type.clone(),
            enrollment_type: enrollment_type.clone(),
            has_boundary: has_boundary.clone(),
            is_cps: is_cps.clone(),
            url: url.clone(),
        }
    }
}

/// One `/staff2` person.
#[derive(Serialize)]
struct StaffRow {
    person_id: i64,
    name: String,
    default_title: String,
    has_email: Option<bool>,
    last_name: Option<String>,
    role_id: Option<String>,
    phone: Option<String>,
    fax: Option<String>,
    email: Option<String>,
}

impl StaffRow {
    fn of(person: &ihsa::StaffPerson) -> Self {
        let ihsa::StaffPerson {
            person_id,
            name,
            default_title,
            has_email,
            last_name,
            role_id,
            phone,
            fax,
            email,
        } = person;
        Self {
            person_id: *person_id,
            name: name.clone(),
            default_title: default_title.clone(),
            has_email: *has_email,
            last_name: last_name.clone(),
            role_id: role_id.clone(),
            phone: phone.clone(),
            fax: fax.clone(),
            email: email.clone(),
        }
    }
}

/// A canonical school with the id it was minted under.
#[derive(Serialize)]
struct SchoolPair {
    school: CanonicalSchool,
    school_id: SchoolId,
}

/// One staff payload's published result: the people as parsed, the coaches they mint, and the
/// retained rows whose payload advertises an address for reveal.
#[derive(Serialize)]
struct StaffFacts {
    staff: Vec<StaffRow>,
    coaches: Vec<CanonicalCoach>,
    paid_reveal_names: Vec<String>,
}

/// The school list parsed into raw rows and canonical schools.
#[derive(Serialize)]
struct DirectoryFacts {
    records: Vec<SchoolRow>,
    schools: Vec<SchoolPair>,
}

/// The canonical school a staff payload is anchored to: `/staff2` carries no school id of its own,
/// so the caller supplies the one the schools payload published.
fn ihsa_anchor_school(records: &[ihsa::SchoolRecord], school_id: &str) -> Result<SchoolPair> {
    let record = records
        .iter()
        .find(|record| record.school_id == school_id)
        .with_context(|| format!("the schools fixture carries school {school_id}"))?;
    let (school, id) = ihsa::parse_school(record, "https://api.ihsa.org/v1/schools", OBSERVED_ON)
        .context("the anchored school has a name")?;
    Ok(SchoolPair {
        school,
        school_id: id,
    })
}

/// Parse one IHSA fixture as the payload its name declares, write its golden, and return the case
/// name with its digest.
fn ihsa_case(file: &str, body: &str) -> Result<(String, String)> {
    let stem = stem_of(file);
    let name = format!("ihsa__{stem}");
    let records = ihsa::parse_schools(&common::fixture("ihsa", "v1_schools.json")?)?;
    if stem.starts_with("v1_schools") {
        let mut schools = Vec::new();
        for record in &records {
            let (school, school_id) =
                ihsa::parse_school(record, "https://api.ihsa.org/v1/schools", OBSERVED_ON)
                    .context("every fixture school has a name")?;
            schools.push(SchoolPair { school, school_id });
        }
        let facts = DirectoryFacts {
            records: records.iter().map(SchoolRow::of).collect(),
            schools,
        };
        return golden_case(&name, &facts);
    }
    if stem.starts_with("staff2_") {
        let staff = ihsa::parse_staff(body)?;
        let anchor = ihsa_anchor_school(&records, "0101")?;
        let mut coaches = Vec::new();
        let mut paid_reveal_names = Vec::new();
        for person in &staff {
            if let Some(coach) = ihsa::parse_coach(
                person,
                &anchor.school_id,
                "https://api.ihsa.org/v1/schools/0101/staff2",
                OBSERVED_ON,
            ) {
                if person.has_email == Some(true) {
                    paid_reveal_names.push(coach.name.clone());
                }
                coaches.push(coach);
            }
        }
        let facts = StaffFacts {
            staff: staff.iter().map(StaffRow::of).collect(),
            coaches,
            paid_reveal_names,
        };
        return golden_case(&name, &facts);
    }
    bail!("fixture {file} matches no IHSA payload kind: v1_schools or staff2_")
}

/// Every committed IHSA fixture, parsed by the payload kind its name declares.
#[test]
fn ihsa_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("ihsa")?;
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let (name, digest) = ihsa_case(&file, &common::fixture("ihsa", &file)?)?;
        cases.insert(name, digest);
    }
    assert_rollup("ihsa", cases, paths.len())
}


/// One row of the KSHSAA directory payload, including the columns the adapter parses but never
/// publishes: the canonical half of this golden is the proof they stay out of entities.
#[derive(Serialize)]
struct DirectoryRow {
    id: u64,
    identifier: String,
    school_name: String,
    mailing_city: String,
    class: Option<String>,
    enrollment: Option<u32>,
    web_site: Option<String>,
    ad_name: Option<String>,
    ad_email: Option<String>,
    ad_cell: Option<String>,
    principal_name: Option<String>,
}

impl DirectoryRow {
    fn of(record: &ks::KshsaaRecord) -> Self {
        let ks::KshsaaRecord {
            id,
            identifier,
            school_name,
            mailing_city,
            class,
            enrollment,
            web_site,
            ad_name,
            ad_email,
            ad_cell,
            principal_name,
        } = record;
        Self {
            id: *id,
            identifier: identifier.clone(),
            school_name: school_name.clone(),
            mailing_city: mailing_city.clone(),
            class: class.clone(),
            enrollment: *enrollment,
            web_site: web_site.clone(),
            ad_name: ad_name.clone(),
            ad_email: ad_email.clone(),
            ad_cell: ad_cell.clone(),
            principal_name: principal_name.clone(),
        }
    }
}

/// The directory's published result: raw rows plus every canonical entity they mint.
#[derive(Serialize)]
struct KshsaaFacts {
    records: Vec<DirectoryRow>,
    schools: Vec<CanonicalSchool>,
    ads: Vec<CanonicalCoach>,
}

/// Parse every committed KSHSAA fixture through the directory endpoint's own path.
#[test]
fn ks_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("ks")?;
    let api = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let stem = stem_of(&file);
        let body = common::fixture("ks", &file)?;
        let records = ks::parse_records(&body)?;
        let mut schools = Vec::new();
        let mut ads = Vec::new();
        for record in &records {
            let Some((school, school_id)) = ks::parse_school(record, api, OBSERVED_ON) else {
                bail!(
                    "{}: every fixture record has a school name",
                    record.identifier
                );
            };
            schools.push(school);
            if let Some(ad) = ks::parse_ad_coach(record, &school_id, api, OBSERVED_ON) {
                ads.push(ad);
            }
        }
        let facts = KshsaaFacts {
            records: records.iter().map(DirectoryRow::of).collect(),
            schools,
            ads,
        };
        let (name, digest) = golden_case(&format!("ks__{stem}"), &facts)?;
        cases.insert(name, digest);
    }
    assert_rollup("ks", cases, paths.len())
}

/// The whole adapter over the seeded directory response.
#[tokio::test]
async fn ks_collect_from_a_seeded_cache_matches_golden() -> Result<()> {
    let url = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
    let dir = tempfile::tempdir().context("temp dir")?;
    let (store, fetcher) = seeded(
        dir.path(),
        &[(url, &common::fixture("ks", "kshsaa_directory_a.json")?)],
    )?;
    let options = ks::Options {
        limit: None,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    };
    let report = ks::collect(&context(&fetcher, &store), &options).await?;

    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    anyhow::ensure!(
        report.requests == 0,
        "the seeded cache answers the one request — left={:?} right={:?}",
        &report.requests,
        &0
    );
    anyhow::ensure!(
        report.from_cache == 1,
        "the directory response is cached — left={:?} right={:?}",
        &report.from_cache,
        &1
    );
    {
        let left_value = &(report.rows, schools.len(), coaches.len());
        let right_value = &(5, 5, 5);
        anyhow::ensure!(left_value == right_value, "five records mint five schools and five athletic directors: {report:?} — left={left_value:?} right={right_value:?}");
    }
    common::assert_golden(
        "ks__collect_directory_a",
        &SchoolsRun {
            report: &report,
            schools: &schools,
            coaches: &coaches,
        },
    )
}


/// One schedule row, with the venue and level resolution the meet carries.
#[derive(Serialize)]
struct MeetRowFacts {
    date: String,
    name: String,
    location: String,
    slug: Option<String>,
    aria_label: Option<String>,
    venue_state: Option<String>,
    level: CompetitionLevel,
}

impl MeetRowFacts {
    fn of(row: &wayzata::MeetRow) -> Self {
        let wayzata::MeetRow {
            date,
            name,
            location,
            slug,
            aria_label,
        } = row;
        Self {
            date: date.clone(),
            name: name.clone(),
            location: location.clone(),
            slug: slug.clone(),
            aria_label: aria_label.clone(),
            venue_state: wayzata::venue_state(location).map(|state| state.code().to_string()),
            level: wayzata::level_of(name),
        }
    }
}

/// Every committed Wayzata schedule, read for its own season.
#[test]
fn wayzata_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("wayzata")?;
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let stem = stem_of(&file);
        let rows = wayzata::schedule_rows(&common::fixture("wayzata", &file)?, SEASON)
            .with_context(|| format!("reading {file}"))?;
        let facts = rows.iter().map(MeetRowFacts::of).collect::<Vec<_>>();
        let (name, digest) = golden_case(&format!("wayzata__{stem}"), &facts)?;
        cases.insert(name, digest);
    }
    assert_rollup("wayzata", cases, paths.len())
}

/// The whole adapter over both seeded schedules: the report, and the meets the store ends up with.
#[tokio::test]
async fn wayzata_collect_from_a_seeded_cache_matches_golden() -> Result<()> {
    let dir = tempfile::tempdir().context("temp dir")?;
    let (store, fetcher) = seeded(
        dir.path(),
        &[
            (
                &wayzata::schedule_url(ScheduleSport::Track, SEASON),
                &common::fixture("wayzata", "track_2026_schedule.html")?,
            ),
            (
                &wayzata::schedule_url(ScheduleSport::CrossCountry, SEASON),
                &common::fixture("wayzata", "xc_2026_schedule.html")?,
            ),
        ],
    )?;
    let options = wayzata::Options {
        years: vec![SEASON],
        limit: None,
        refresh: false,
        observed_on: Some(OBSERVED_ON.to_string()),
    };
    let report = wayzata::collect(&context(&fetcher, &store), &options).await?;

    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    anyhow::ensure!(
        report.requests == 0,
        "both schedules are answered from cache — left={:?} right={:?}",
        &report.requests,
        &0
    );
    anyhow::ensure!(
        report.from_cache == 2,
        "one cached page per sport — left={:?} right={:?}",
        &report.from_cache,
        &2
    );
    anyhow::ensure!(
        report.rows == 23,
        "13 track rows plus 10 cross-country rows: {report:?} — left={:?} right={:?}",
        &report.rows,
        &23
    );
    anyhow::ensure!(
        meets.len() >= 20,
        "most schedule rows mint a distinct meet; the store holds {}",
        meets.len()
    );
    common::assert_golden(
        "wayzata__collect_2026_schedules",
        &MeetsRun {
            report: &report,
            meets: &meets,
        },
    )
}


/// One parsed result file.
#[derive(Serialize)]
struct MeetFacts {
    name: String,
    date: String,
    end_date: Option<String>,
    timer: Option<String>,
    rows_parsed: usize,
    rows_skipped: usize,
    events: Vec<EventFacts>,
}

impl MeetFacts {
    fn of(meet: &compiled::ParsedMeet) -> Self {
        let compiled::ParsedMeet {
            name,
            date,
            end_date,
            timer,
            events,
            rows_parsed,
            rows_skipped,
        } = meet;
        Self {
            name: name.clone(),
            date: date.clone(),
            end_date: end_date.clone(),
            timer: timer.clone(),
            rows_parsed: *rows_parsed,
            rows_skipped: *rows_skipped,
            events: events.iter().map(EventFacts::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct EventFacts {
    label: String,
    kind: EventKind,
    gender: Gender,
    division: Option<String>,
    round: Option<String>,
    rows: Vec<RowFacts>,
}

impl EventFacts {
    fn of(event: &compiled::ParsedEvent) -> Self {
        let compiled::ParsedEvent {
            label,
            kind,
            gender,
            division,
            round,
            rows,
        } = event;
        Self {
            label: label.clone(),
            kind: kind.clone(),
            gender: *gender,
            division: division.clone(),
            round: round.clone(),
            rows: rows.iter().map(RowFacts::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct RowFacts {
    place: Option<u16>,
    name: String,
    grade: Option<Grade>,
    school: String,
    mark: Mark,
    wind_mps: Option<f64>,
    heat: Option<String>,
    points: Option<f64>,
    legs: Vec<LegFacts>,
}

impl RowFacts {
    fn of(row: &compiled::ParsedRow) -> Self {
        let compiled::ParsedRow {
            place,
            name,
            grade,
            school,
            mark,
            wind_mps,
            heat,
            points,
            legs,
        } = row;
        Self {
            place: *place,
            name: name.clone(),
            grade: *grade,
            school: school.clone(),
            mark: mark.clone(),
            wind_mps: *wind_mps,
            heat: heat.clone(),
            points: *points,
            legs: legs.iter().map(LegFacts::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct LegFacts {
    position: u8,
    name: String,
    grade: Option<Grade>,
}

impl LegFacts {
    fn of(leg: &compiled::RelayLeg) -> Self {
        let compiled::RelayLeg {
            position,
            name,
            grade,
        } = leg;
        Self {
            position: *position,
            name: name.clone(),
            grade: *grade,
        }
    }
}

/// The 2026 regional export shape, verbatim from `compiled.rs`'s own test corpus: two event blocks
/// per page, a relay on the left and a preliminary heat on the right that carries a qualifier
/// letter instead of points.
const REGIONAL_EXPORT: &str = r#"
05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North
                                                        Appleton North HS  Tue, May 26, 2026
                                                                     Results
Girls' 4x800 Relay Division 1                     Finals                    Girls' 100 Meters Division 1              Prelims
       Team                    Relay        Finals             Pts               Athlete                 Yr Team              Prelims
1      HORTONVILLE             'A'          9:55.11            10           1    Parrish, Ashley         11   APPLETON NOR…   12.30 Q
    1) Wloszczynski, Lexi 10         2) Young, Ellie 9                      2    Thompson, Emily         12   APPLETON NOR…   12.74 q
    3) Falbo, Hailey 12              4) Huza, Hannah 12                     3    Rades, Jayla            11   HORTONVILLE     12.83 Q
                                                                            4    Rezash, Johannah        12   WEST DE PERE    13.12 q
2      APPLETON NORTH          'A'          9:56.50            8
                                                                            5    Lopez, Eilianyz         10   WEST DE PERE    13.38 q
    1) Dehlinger, Audry 11           2) Brazzale, Elise 10                  6    Hammen, Allie           9    APPLETON WEST   13.39 q
    3) Busch, Sophia 12              4) Helmbrecht, Ava 12
                                                                            7    Josephson, Sydney       9    KAUKAUNA        13.44 q
3      KIMBERLY                'A'          10:03.38           6            8    Olson, Denise           12   APPLETON EAST   13.55 q
"#;

/// A page stamp that also carries the meet name, repeated by the header line beneath it — and no
/// event block, which is what the reader requires before it claims a page.
const PAGE_STAMP_EXPORT: &str = "5/27/25, 8:35 PM                                              Manage D3 Regional 4B - Deerfield\n\
                          D3 Regional 4B - Deerfield\n\
                     Deerfield HS Track   Tue, May 27, 2025\n\
                                  Results\n";

/// A body with no header at all: the compiled reader must not claim it.
const HEADERLESS_EXPORT: &str = "Girls' 100 Meters Division 1   Finals";

/// How many documented export bodies this source contributes (it has no fixture corpus).
const COMPILED_EXPORTS: usize = 3;

/// The compiled reader on the export shapes its own tests document. There is no committed fixture
/// for this layout — the reader is reached only as the PDF fallback behind Hy-Tek, and every
/// committed result body belongs to a layout an earlier parser claims.
///
/// The reader states two boundaries besides the export itself: a body with a readable header but no
/// event row is declined (`parse` publishes only meets with rows), and a body with no header at all
/// is not claimed at all. Both come back as `null` here on purpose, so a later reader that starts
/// claiming them fails this run.
#[test]
fn compiled_documented_exports_match_golden() -> Result<()> {
    let source = SourceRef::new("wiaa_results", None);
    let mut cases = BTreeMap::new();

    let regional = compiled::parse(
        &hytek::lines_from_pdf_text(REGIONAL_EXPORT),
        source.clone(),
        SEASON,
    );
    let regional = regional.context("the regional export has a meet header")?;
    anyhow::ensure!(
        regional.name == "D1 Regional 8B - Appleton North",
        "left={:?} right={:?}",
        &regional.name,
        &"D1 Regional 8B - Appleton North"
    );
    anyhow::ensure!(
        regional.date == "2026-05-26",
        "left={:?} right={:?}",
        &regional.date,
        &"2026-05-26"
    );
    anyhow::ensure!(
        regional.events.len() == 2,
        "one event per block — left={:?} right={:?}",
        &regional.events.len(),
        &2
    );
    let (name, digest) = golden_case("compiled__regional_export", &MeetFacts::of(&regional))?;
    cases.insert(name, digest);

    let page_stamp = compiled::parse(
        &hytek::lines_from_pdf_text(PAGE_STAMP_EXPORT),
        source.clone(),
        SEASON,
    );
    let (name, digest) = golden_case(
        "compiled__page_stamp_only_export",
        &page_stamp.as_ref().map(MeetFacts::of),
    )?;
    cases.insert(name, digest);

    let headerless = compiled::parse(
        &hytek::lines_from_pdf_text(HEADERLESS_EXPORT),
        source,
        SEASON,
    );
    let (name, digest) = golden_case(
        "compiled__headerless_export",
        &headerless.as_ref().map(MeetFacts::of),
    )?;
    cases.insert(name, digest);

    assert_rollup("compiled", cases, COMPILED_EXPORTS)
}
