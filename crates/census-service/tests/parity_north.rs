//! Golden-corpus parity for the MSHSL, ND/NSAA (`plain_names`) and Hy-Tek adapters.
//!
//! A decomposition refactor moves code between functions and files; these tests are the proof that
//! no published parse result changes while it happens. Every fixture in `tests/fixtures/mshsl` and
//! `tests/fixtures/plain_names` is walked with `common::fixtures` and asserted per file — a new
//! fixture without a case fails the run instead of becoming a silent coverage hole — and the Hy-Tek
//! report parser is pinned against every result file its own `#[cfg(test)]` tests read.
//!
//! The two `collect` surfaces are driven end to end against a seeded disk cache, so their goldens
//! cover the `AdapterReport` and every entity the run appends to a `tempfile` store, not only the
//! parse helpers. Each source directory also goldens a per-file digest list (`<source>__corpus`):
//! one case drifting turns the aggregate red even if that case's own golden were refreshed by
//! mistake.
//!
//! Seed once with `GOLDEN_UPDATE=1 cargo nextest run -p census-service --test parity_north`, review
//! the diff, then re-run without the variable. The comparison is on golden bytes, so a changed
//! field, ordering or default fails.

mod common;

use census_domain::UsJurisdiction;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use census_crawl::mshsl::{self, SchoolDetail, SchoolListRow, COACH_API_PREFIX, TEAMS_VIEW_URL};
use census_crawl::net::Fetcher;
use census_crawl::plain_names::{self, NsaaRow, NsaaSchool};
use census_crawl::result_file::ParsedMeet;
use census_crawl::{hytek, AdapterContext};
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, Gender, Grade, SchoolId, SchoolYear, SourceRef, Sport,
};
use census_store::{Store, Table};
use serde_json::{json, Value};

/// Evidence date every case stamps, matching the adapters' own fixture-backed tests.
const OBSERVED_ON: &str = "2026-09-20";
/// Source directories under `tests/fixtures`.
const MSHSL: &str = "mshsl";
const PLAIN_NAMES: &str = "plain_names";
const WIAA_RESULTS: &str = "wiaa_results";

/// Fixture file name without its extension: the golden suffix.
fn stem(name: &str) -> Result<String> {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .with_context(|| format!("fixture {name} has no file stem"))
}

fn read_fixture(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))
}

/// Golden names are unique inside a corpus.
///
/// Two fixtures whose names collapse onto one golden would silently pin only the last write —
/// `d1boysstateresults-dash.htm` and `.txt` shared a stem until the Hy-Tek case was named by file —
/// so the claim is checked rather than trusted.
fn claim_golden(claimed: &mut Vec<String>, name: &str) -> Result<()> {
    ensure!(
        !claimed.iter().any(|seen| seen == name),
        "{name}: two fixtures claim one golden — name them apart"
    );
    claimed.push(name.to_string());
    Ok(())
}

/// Seed the fetcher's on-disk cache for `url` under the key `Fetcher` derives
/// (`sha256(method \x1f url \x1f body)[..16]`), so `collect` can be driven without a socket.
fn seed_cache(cache_dir: &Path, url: &str, body: &str) -> Result<()> {
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
    let meta = json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T14:39:00Z",
    });
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating cache dir {}", cache_dir.display()))?;
    fs::write(cache_dir.join(format!("{key}.body")), body)
        .with_context(|| format!("writing the cached body for {url}"))?;
    fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta).context("serializing cache metadata")?,
    )
    .with_context(|| format!("writing the cached metadata for {url}"))?;
    Ok(())
}


/// Every fixture in `tests/fixtures/mshsl`, one golden plus one aggregate digest per file.
#[test]
fn mshsl_fixtures_match_golden() -> Result<()> {
    let listing = common::fixture(MSHSL, "schools_listing.html")?;
    let rows = mshsl::parse_school_list(&listing);
    let mut corpus: Vec<Value> = Vec::new();
    let mut claimed: Vec<String> = Vec::new();
    for path in common::fixtures(MSHSL)? {
        let name = common::file_name(&path)?;
        let body = read_fixture(&path)?;
        let value = mshsl_case(&name, &body, &rows)?;
        let golden = format!("mshsl__{}", stem(&name)?);
        claim_golden(&mut claimed, &golden)?;
        common::assert_golden(&golden, &value)?;
        corpus.push(json!({ "fixture": name, "digest": common::digest(&value)? }));
    }
    common::assert_golden("mshsl__corpus", &corpus)?;
    Ok(())
}

/// Dispatch one fixture onto its parity case; an unhandled fixture is a coverage hole, not a skip.
fn mshsl_case(name: &str, body: &str, rows: &[SchoolListRow]) -> Result<Value> {
    if name.starts_with("schools_listing") {
        return Ok(mshsl_listing(body));
    }
    if let Some(slug) = name
        .strip_prefix("school_detail_")
        .and_then(|rest| rest.strip_suffix(".html"))
    {
        return mshsl_detail(slug, body, rows);
    }
    if name.starts_with("team_nodes_") {
        return Ok(mshsl_team_nodes(body));
    }
    if let Some(case) = name
        .strip_prefix("coach_records_")
        .and_then(|rest| rest.strip_suffix(".json"))
    {
        return mshsl_coach_records(case, body, rows);
    }
    bail!("{name}: no parity case — every mshsl fixture has to be asserted")
}

/// A fixture with no parity case has to fail the run, not quietly drop out of the corpus.
#[test]
fn unmapped_fixture_names_are_rejected() -> Result<()> {
    ensure!(
        mshsl_case("brand_new_fixture.html", "", &[]).is_err(),
        "an unmapped mshsl fixture must be an error"
    );
    ensure!(
        plain_names_case("brand_new_fixture.html", "", "").is_err(),
        "an unmapped plain_names fixture must be an error"
    );
    Ok(())
}

/// The school listing: its rows, its pager and the URL each row page takes.
fn mshsl_listing(body: &str) -> Value {
    let rows = mshsl::parse_school_list(body);
    let next = mshsl::parse_next_listing_page(body, 0);
    json!({
        "next_page": next,
        "next_page_url": next.map(mshsl::listing_page_url),
        "rows": rows
            .iter()
            .map(|row| json!({
                "slug": row.slug,
                "name": row.name,
                "city": row.city,
                "page_url": mshsl::school_page_url(&row.slug),
            }))
            .collect::<Vec<Value>>(),
    })
}

/// The listing row a school page belongs to.
///
/// Foley and Wayzata sit on listing pages outside this fixture corpus; their row is the one the
/// listing would publish (the page's own `<h1>`), without a city no fixture carries.
fn row_for(slug: &str, detail: &SchoolDetail, rows: &[SchoolListRow]) -> SchoolListRow {
    match rows.iter().find(|row| row.slug == slug) {
        Some(row) => row.clone(),
        None => SchoolListRow {
            slug: slug.to_string(),
            name: detail.name.clone().unwrap_or_default(),
            city: None,
        },
    }
}

fn detail_json(detail: &SchoolDetail) -> Value {
    json!({
        "name": detail.name,
        "school_id": detail.school_id,
        "enrollment": detail.enrollment,
        "website": detail.website,
        "admin": detail
            .admin
            .iter()
            .map(|entry| json!({
                "role": entry.role,
                "name": entry.name,
                "emails": entry.emails,
                "ad_role": mshsl::ad_role(&entry.role),
            }))
            .collect::<Vec<Value>>(),
    })
}

/// One school page: its facts, every Administration entry, the roles those labels map to, the
/// canonical school and the AD rows the page emits.
fn mshsl_detail(slug: &str, body: &str, rows: &[SchoolListRow]) -> Result<Value> {
    let detail = mshsl::parse_school_detail(body);
    let row = row_for(slug, &detail, rows);
    let page_url = mshsl::school_page_url(slug);
    let (school, school_id) = match mshsl::school_entities(&row, &detail, &page_url, OBSERVED_ON) {
        Some((school, school_id)) => (Some(school), Some(school_id)),
        None => (None, None),
    };
    let school_key = mshsl::provider_key(&row, &detail);
    let ad_coaches = match &school_id {
        Some(school_id) => {
            mshsl::ad_coaches(&detail, school_id, &school_key, &page_url, OBSERVED_ON)
        }
        None => Vec::new(),
    };
    Ok(json!({
        "row": { "slug": row.slug, "name": row.name, "city": row.city },
        "school_key": school_key,
        "school_id": &school_id,
        "school": &school,
        "detail": detail_json(&detail),
        "domains": mshsl::school_domains(&detail),
        "ad_coaches": &ad_coaches,
    }))
}

fn team_node_json(node: &mshsl::TeamNode) -> Value {
    json!({
        "nid": node.nid,
        "title": node.title,
        "alias": node.alias,
        "page_url": node.page_url(),
    })
}

fn sport_json(sport: Option<(Sport, Gender)>) -> Value {
    match sport {
        Some((sport, gender)) => json!({ "sport": sport, "gender": gender }),
        None => Value::Null,
    }
}

/// The team list: every node, the track/XC subset `collect` walks, and the sport each alias maps to.
fn mshsl_team_nodes(body: &str) -> Value {
    let nodes = mshsl::parse_team_nodes(body);
    json!({
        "nodes": nodes.iter().map(team_node_json).collect::<Vec<Value>>(),
        "selected": mshsl::select_team_nodes(&nodes)
            .iter()
            .map(team_node_json)
            .collect::<Vec<Value>>(),
        "classification": nodes
            .iter()
            .map(|node| json!({
                "alias": node.alias,
                "sport": sport_json(mshsl::team_sport(&node.alias)),
            }))
            .collect::<Vec<Value>>(),
    })
}

/// Each coach payload, the school page and team list it belongs to, and the alias marker naming its
/// team.
const COACH_CASES: [(&str, &str, &str, &str); 3] = [
    (
        "aitkin_track-and-field-boys",
        "school_detail_aitkin-high-school.html",
        "team_nodes_aitkin.json",
        "track-and-field-boys",
    ),
    (
        "aitkin_track-and-field-girls",
        "school_detail_aitkin-high-school.html",
        "team_nodes_aitkin.json",
        "track-and-field-girls",
    ),
    (
        "wayzata_track_boys",
        "school_detail_wayzata-high-school.html",
        "team_nodes_wayzata.json",
        "track-and-field-boys",
    ),
];

/// One `/api/coaches/<nid>` payload: every record, the level/role decision taken on it, the email
/// the domain rule keeps, and the canonical coach rows the payload mints.
fn mshsl_coach_records(case: &str, body: &str, rows: &[SchoolListRow]) -> Result<Value> {
    let binding = COACH_CASES.iter().find(|(name, ..)| *name == case).copied();
    let Some((_, detail_file, nodes_file, marker)) = binding else {
        bail!("{case}: coach payload has no school/team binding — add it to COACH_CASES");
    };
    let slug = detail_file
        .strip_prefix("school_detail_")
        .and_then(|rest| rest.strip_suffix(".html"))
        .context("the bound detail fixture names a slug")?;
    let detail_body = common::fixture(MSHSL, detail_file)?;
    let detail = mshsl::parse_school_detail(&detail_body);
    let row = row_for(slug, &detail, rows);
    let page_url = mshsl::school_page_url(slug);
    let (_school, school_id) = mshsl::school_entities(&row, &detail, &page_url, OBSERVED_ON)
        .context("the bound school page mints a canonical school")?;

    let nodes_body = common::fixture(MSHSL, nodes_file)?;
    let nodes = mshsl::parse_team_nodes(&nodes_body);
    let node = nodes
        .iter()
        .find(|node| node.alias.contains(marker))
        .with_context(|| format!("{marker}: the bound team list publishes this team"))?;
    let api_url = format!("{COACH_API_PREFIX}{}", node.nid);
    let records = mshsl::parse_coach_records(body);
    let teams = vec![mshsl::TeamCoaches {
        node: node.clone(),
        api_url: api_url.clone(),
        records: records.clone(),
    }];

    let domains = mshsl::school_domains(&detail);
    let entities = mshsl::coach_entities(&teams, &school_id, &domains, OBSERVED_ON);
    Ok(json!({
        "slug": slug,
        "school_id": &school_id,
        "domains": &domains,
        "api_url": api_url,
        "node": team_node_json(node),
        "records": records
            .iter()
            .map(|record| json!({
                "name": record.name,
                "level": record.level,
                "email": record.email,
                "published_level": mshsl::is_published_level(&record.level),
                "role": mshsl::coach_role(&record.level),
                "accepted_email": record
                    .email
                    .as_deref()
                    .and_then(|address| mshsl::published_coach_email(address, &domains)),
            }))
            .collect::<Vec<Value>>(),
        "entities": &entities,
    }))
}

/// `mshsl::collect` against a seeded cache: the report plus every row it appends to a fresh store.
#[tokio::test]
async fn mshsl_collect_matches_golden() -> Result<()> {
    let dir = tempfile::tempdir().context("temp dir")?;
    let cache = dir.path().join("http");
    let listing_body = common::fixture(MSHSL, "schools_listing_first_page.html")?;
    let detail_body = common::fixture(MSHSL, "school_detail_aitkin-high-school.html")?;
    let teams_body = common::fixture(MSHSL, "team_nodes_aitkin.json")?;
    let detail = mshsl::parse_school_detail(&detail_body);
    let rows = mshsl::parse_school_list(&listing_body);
    let row = rows
        .iter()
        .find(|row| row.slug == "aitkin-high-school")
        .context("the first listing page publishes Aitkin")?;
    let school_key = mshsl::provider_key(row, &detail);
    let teams_url = format!(
        "{TEAMS_VIEW_URL}?views-argument%5B%5D={school_key}&fields%5Bnode--participant%5D=title,path,drupal_internal__nid"
    );
    seed_cache(&cache, &mshsl::listing_page_url(0), &listing_body)?;
    seed_cache(
        &cache,
        &mshsl::school_page_url("aitkin-high-school"),
        &detail_body,
    )?;
    seed_cache(&cache, &teams_url, &teams_body)?;
    for node in mshsl::select_team_nodes(&mshsl::parse_team_nodes(&teams_body)) {
        let fixture = if node.alias.contains("track-and-field-boys") {
            "coach_records_aitkin_track-and-field-boys.json"
        } else if node.alias.contains("track-and-field-girls") {
            "coach_records_aitkin_track-and-field-girls.json"
        } else {
            continue;
        };
        seed_cache(
            &cache,
            &format!("{COACH_API_PREFIX}{}", node.nid),
            &common::fixture(MSHSL, fixture)?,
        )?;
    }

    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?;
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    };
    let options = mshsl::Options {
        limit: Some(1),
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: vec!["Aitkin High School".to_string()],
    };
    let report = mshsl::collect(&ctx, &options).await?;
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
    ensure!(
        report.requests == 0,
        "the seeded cache serves every response, {} request(s) left the process",
        report.requests
    );
    common::assert_golden(
        "mshsl__collect",
        &json!({ "report": &report, "schools": &schools, "coaches": &coaches }),
    )
}


/// Every fixture in `tests/fixtures/plain_names`, one golden plus an aggregate digest per file.
#[test]
fn plain_names_fixtures_match_golden() -> Result<()> {
    let index = common::fixture(PLAIN_NAMES, "nd_schools_index.html")?;
    let mut corpus: Vec<Value> = Vec::new();
    let mut claimed: Vec<String> = Vec::new();
    for path in common::fixtures(PLAIN_NAMES)? {
        let name = common::file_name(&path)?;
        let body = read_fixture(&path)?;
        let value = plain_names_case(&name, &body, &index)?;
        let golden = format!("plain_names__{}", stem(&name)?);
        claim_golden(&mut claimed, &golden)?;
        common::assert_golden(&golden, &value)?;
        corpus.push(json!({ "fixture": name, "digest": common::digest(&value)? }));
    }
    common::assert_golden("plain_names__corpus", &corpus)?;
    Ok(())
}

/// Dispatch one fixture onto its parity case; an unhandled fixture is a coverage hole, not a skip.
fn plain_names_case(name: &str, body: &str, index: &str) -> Result<Value> {
    match name {
        "nd_schools_index.html" => plain_names_nd_index(body),
        "nd_school_page.html" => plain_names_nd_page("1045", body, index),
        "nd_school_page_no_ad.html" => plain_names_nd_page("1378", body, index),
        "nsaa_directory_form.html" => plain_names_nsaa_form(body),
        "nsaa_directory_export.html" | "nsaa_school_get_adams_central.html" => {
            plain_names_nsaa_directory(body)
        }
        other => bail!("{other}: no parity case — every plain_names fixture has to be asserted"),
    }
}

/// The NDHSAA member-school index: every published ref and the page URL it resolves to.
fn plain_names_nd_index(body: &str) -> Result<Value> {
    let members = plain_names::parse_nd_school_refs(body)?;
    Ok(json!({
        "members": members
            .iter()
            .map(|member| json!({
                "id": member.id,
                "slug": member.slug,
                "url": member.url(),
            }))
            .collect::<Vec<Value>>(),
    }))
}

/// One NDHSAA school page: the parsed school, its staff lines with the role each maps to, its
/// offerings with the sport each maps to, and the coach/AD rows it mints.
fn plain_names_nd_page(id: &str, body: &str, index: &str) -> Result<Value> {
    let members = plain_names::parse_nd_school_refs(index)?;
    let member = members
        .iter()
        .find(|member| member.id == id)
        .with_context(|| format!("the index lists member school {id}"))?;
    let url = member.url();
    let staff = plain_names::parse_nd_staff(body)?;
    let offerings = plain_names::parse_nd_offerings(body)?;
    let (school, school_id) = match plain_names::parse_nd_school_page(body, member, OBSERVED_ON)? {
        Some((school, school_id)) => (Some(school), Some(school_id)),
        None => (None, None),
    };
    let ad_coaches = match &school_id {
        Some(school_id) => plain_names::nd_ad_coaches(&staff, school_id, &url, OBSERVED_ON),
        None => Vec::new(),
    };
    let sport_coaches = match &school_id {
        Some(school_id) => plain_names::nd_sport_coaches(&offerings, school_id, &url, OBSERVED_ON),
        None => Vec::new(),
    };
    Ok(json!({
        "member": { "id": member.id, "slug": member.slug, "url": url },
        "school_id": &school_id,
        "school": &school,
        "staff": staff
            .iter()
            .map(|role| json!({
                "label": role.label,
                "name": role.name,
                "role": plain_names::parse_nd_role(&role.label),
            }))
            .collect::<Vec<Value>>(),
        "offerings": offerings
            .iter()
            .map(|offering| json!({
                "label": offering.label,
                "coaches": offering.coaches,
                "co_op": offering.co_op,
                "sport": sport_json(plain_names::parse_nd_sport(&offering.label)),
            }))
            .collect::<Vec<Value>>(),
        "ad_coaches": &ad_coaches,
        "sport_coaches": &sport_coaches,
    }))
}

/// The NSAA directory form's option list: the member-school key space and the request URL each name
/// resolves to.
fn plain_names_nsaa_form(body: &str) -> Result<Value> {
    let names = plain_names::parse_nsaa_school_names(body)?;
    Ok(json!({
        "names": &names,
        "urls": names
            .iter()
            .map(|name| plain_names::nsaa_school_url(name))
            .collect::<Vec<String>>(),
    }))
}

fn nsaa_row_json(row: Option<NsaaRow>) -> Value {
    match row {
        Some(NsaaRow::SportCoach { sport, gender }) => {
            json!({ "kind": "sport_coach", "sport": sport, "gender": gender })
        }
        Some(NsaaRow::AthleticDirector) => json!({ "kind": "athletic_director" }),
        None => Value::Null,
    }
}

fn nsaa_school_json(
    school: &NsaaSchool,
    url: &str,
    canonical: &CanonicalSchool,
    school_id: &SchoolId,
    coaches: &[CanonicalCoach],
) -> Value {
    json!({
        "url": url,
        "name": school.name,
        "city": school.city,
        "enrollment": school.enrollment,
        "homepage": school.homepage,
        "school_id": school_id,
        "school": canonical,
        "roles": school
            .roles
            .iter()
            .map(|role| json!({
                "label": role.label,
                "name": role.name,
                "co_op": role.co_op,
                "row": nsaa_row_json(plain_names::parse_nsaa_row(&role.label)),
            }))
            .collect::<Vec<Value>>(),
        "coaches": coaches,
    })
}

/// One NSAA directory response (the bulk screen's first blocks, or one school's view): every school
/// block with its roles, its canonical school and its coach rows.
fn plain_names_nsaa_directory(body: &str) -> Result<Value> {
    let schools = plain_names::parse_nsaa_directory(body)?;
    let mut entries: Vec<Value> = Vec::with_capacity(schools.len());
    for school in &schools {
        let url = plain_names::nsaa_school_url(&school.name);
        let (canonical, school_id) = plain_names::parse_nsaa_school(school, &url, OBSERVED_ON);
        let coaches = plain_names::nsaa_coaches(school, &school_id, &url, OBSERVED_ON)?;
        entries.push(nsaa_school_json(
            school, &url, &canonical, &school_id, &coaches,
        ));
    }
    Ok(json!({ "schools": entries }))
}

/// `plain_names::collect` against a seeded cache: one ND school page and one NSAA school view.
///
/// The NDHSAA walk normally pages through all 169 member schools; every one without a fixture page
/// is marked done in the journal first, which is exactly the state a resumed run finds, so the walk
/// fetches the single page this corpus carries and still runs its real journal, parse and append
/// path. Nebraska is capped at one school by `options.limit`.
#[tokio::test]
async fn plain_names_collect_matches_golden() -> Result<()> {
    let dir = tempfile::tempdir().context("temp dir")?;
    let cache = dir.path().join("http");
    let index = common::fixture(PLAIN_NAMES, "nd_schools_index.html")?;
    let members = plain_names::parse_nd_school_refs(&index)?;
    let sheyenne = members
        .iter()
        .find(|member| member.id == "1045")
        .context("the index lists West Fargo Sheyenne")?;
    seed_cache(&cache, plain_names::ND_SCHOOLS_URL, &index)?;
    seed_cache(
        &cache,
        &sheyenne.url(),
        &common::fixture(PLAIN_NAMES, "nd_school_page.html")?,
    )?;
    seed_cache(
        &cache,
        plain_names::NSAA_FORM_URL,
        &common::fixture(PLAIN_NAMES, "nsaa_directory_form.html")?,
    )?;
    seed_cache(
        &cache,
        &plain_names::nsaa_school_url("Adams Central"),
        &common::fixture(PLAIN_NAMES, "nsaa_school_get_adams_central.html")?,
    )?;

    let store = Store::open(dir.path().join("store"))?;
    let seeded = json!({ "seeded": true });
    for member in members.iter().filter(|member| member.id != "1045") {
        let key = format!("ND:{}", member.id);
        store.journal_done("ndhsaa_schools", &key, &seeded)?;
        store.journal_done("ndhsaa_coaches", &key, &seeded)?;
    }

    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?;
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    };
    let options = plain_names::Options {
        limit: Some(1),
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: vec![UsJurisdiction::NorthDakota, UsJurisdiction::Nebraska],
        school_names: Vec::new(),
    };
    let report = plain_names::collect(&ctx, &options).await?;
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
    ensure!(
        report.requests == 0,
        "the seeded cache serves every response, {} request(s) left the process",
        report.requests
    );
    ensure!(
        report.errors == 0,
        "both selected providers walked cleanly, {} error(s): {:?}",
        report.errors,
        report.notes
    );
    common::assert_golden(
        "plain_names__collect",
        &json!({ "report": &report, "schools": &schools, "coaches": &coaches }),
    )
}


/// The Hy-Tek fixtures its own `#[cfg(test)]` tests read, with the line splitter each needs.
///
/// `racinesectionalb-finish-list.htm` shares the directory but belongs to the RaceDay parser, so it
/// is not part of this corpus.
const HYTEK_FIXTURES: [(&str, &str); 5] = [
    ("d1boysstateresults-dash.htm", "html"),
    ("d1boysstateresults-sections.htm", "html"),
    ("d1boysstateresults-dash.txt", "text"),
    ("trackside-regional.htm", "html"),
    ("seed-column-regional.htm", "html"),
];

fn meet_json(meet: &ParsedMeet) -> Value {
    json!({
        "name": meet.name,
        "date": meet.date,
        "end_date": meet.end_date,
        "timer": meet.timer,
        "rows_parsed": meet.rows_parsed,
        "rows_skipped": meet.rows_skipped,
        "events": meet
            .events
            .iter()
            .map(|event| json!({
                "label": event.label,
                "kind": event.kind,
                "gender": event.gender,
                "division": event.division,
                "round": event.round,
                "rows": event
                    .rows
                    .iter()
                    .map(|row| json!({
                        "place": row.place,
                        "name": row.name,
                        "grade": row.grade.map(Grade::get),
                        "school": row.school,
                        "mark": row.mark,
                        "wind_mps": row.wind_mps,
                        "heat": row.heat,
                        "points": row.points,
                        "legs": row
                            .legs
                            .iter()
                            .map(|leg| json!({
                                "position": leg.position,
                                "name": leg.name,
                                "grade": leg.grade.map(Grade::get),
                            }))
                            .collect::<Vec<Value>>(),
                    }))
                    .collect::<Vec<Value>>(),
            }))
            .collect::<Vec<Value>>(),
    })
}

/// The Hy-Tek report parser on every fixture it reads: the report lines, the meet, and the digest
/// list that turns any single drifted case into a failing aggregate.
#[test]
fn hytek_fixtures_match_golden() -> Result<()> {
    let mut corpus: Vec<Value> = Vec::new();
    let mut claimed: Vec<String> = Vec::new();
    for (file, shape) in HYTEK_FIXTURES {
        let body = common::fixture(WIAA_RESULTS, file)?;
        let lines = match shape {
            "html" => hytek::lines_from_html(&body),
            _ => hytek::lines_from_text(&body),
        };
        let meet = hytek::parse(&lines, SourceRef::new("wiaa_results", None));
        let value = json!({
            "fixture": file,
            "shape": shape,
            "lines": lines.len(),
            "meet": meet.as_ref().map(meet_json),
        });
        let golden = format!("hytek__{file}");
        claim_golden(&mut claimed, &golden)?;
        common::assert_golden(&golden, &value)?;
        corpus.push(json!({ "fixture": file, "digest": common::digest(&value)? }));
    }
    common::assert_golden("hytek__corpus", &corpus)?;
    Ok(())
}
