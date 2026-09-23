//! Golden-corpus parity for the source-artifact slice of the census: the AthleticLIVE harvest and
//! athlete adapters, the MileSplit HTML adapter, the coach-contact CSV and the Athletic.net bio
//! surface.
//!
//! A decomposition refactor may move code between functions and files, but it may not change a
//! single published parse result. Every case below parses committed bytes and compares the
//! serialized result byte-for-byte with `tests/golden/<source>__<case>.json`, then hashes the whole
//! case list into one aggregate digest per source. The digest is the coverage guard: a case that
//! stops being asserted, or a fixture that stops being walked, changes the digest and fails the
//! test instead of silently shrinking the corpus.
//!
//! Fixtures walked here:
//!
//! * `tests/fixtures/athleticlive/` — every file, through `parse_meets_csv` → `build_meets` and
//!   through the adapter's own `collect` over a scratch store.
//! * `tests/fixtures/athleticlive_athletes/` — every file, through `meet_targets` / `batch_query` /
//!   `build_entities` exactly the way the adapter's `#[cfg(test)]` module reads it.
//! * `tests/fixtures/milesplit/` — every file, through `parse_team_index` / `parse_roster` /
//!   `roster_entities`, plus `fetch_team_index` / `fetch_roster` served from the seeded cache.
//! * `tests/fixtures/coach_contacts_sample.csv` — the fixtures root's `coach_contacts*` captures,
//!   through `row_entities` and `import_csv`.
//! * `crates/census-crawl/src/athleticnet.rs` — no fixture directory exists for this adapter, so its cases
//!   replay the registry, mark tokens and bio payloads its own `#[cfg(test)]` module captures
//!   inline. No fixture file is invented for it.
//!
//! Seeding: `GOLDEN_UPDATE=1 cargo nextest run -p census-service --test parity_national` rewrites
//! the goldens; the comparison run never sets it.

mod common;

use anyhow::{bail, Context, Result};
use census_crawl::athleticlive_athletes::{self, AthleteHit, MeetTarget};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{athleticlive, athleticnet, coach_contacts, milesplit, AdapterContext};
use census_domain::model::{
    CanonicalMeet, CompetitionLevel, EventKind, SchoolYear, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Observation date and school year the adapters are driven with; every golden below encodes them.
const OBSERVED_ON: &str = "2026-09-20";
const SCHOOL_YEAR: SchoolYear = SchoolYear::new(2026).expect("2026 is a season");

// -------------------------------------------------------------------------------------------------
// Harness
// -------------------------------------------------------------------------------------------------

/// A scratch store plus a fetcher over its HTTP cache — the same wiring `main` builds.
struct Harness {
    store: Store,
    fetcher: Fetcher,
    cache: PathBuf,
    /// Declared last: the store closes before the directory that owns it disappears.
    _root: tempfile::TempDir,
}

impl Harness {
    fn new() -> Result<Self> {
        let root = tempfile::tempdir().context("temp dir for a parity store")?;
        let store = Store::open(root.path()).context("opening a scratch store")?;
        let cache = store.http_cache_dir();
        let fetcher = Fetcher::new(&cache, None, Duration::ZERO, HashMap::new(), Vec::new())
            .context("building a fetcher over the scratch cache")?;
        Ok(Self {
            store,
            fetcher,
            cache,
            _root: root,
        })
    }

    fn adapter_context(&self) -> AdapterContext<'_> {
        AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SCHOOL_YEAR,
            observed_on: OBSERVED_ON.to_string(),
        }
    }

    /// Serve `body` for `method url` out of the fetcher's own cache, so an adapter that fetches
    /// still runs against committed bytes with no socket ever opening: the fetcher is cache-first
    /// and returns a cached 200 before it parses the URL, checks robots or takes a host turn.
    ///
    /// The key mirrors the cache's on-disk contract — SHA-256 of `method`, `url` and the request
    /// body joined by `0x1f`, truncated to the leading 16 bytes in hex. A drift in that key makes
    /// the fetch a live request, which the callers below detect via `FetchStats::requests`.
    fn seed(&self, method: &str, url: &str, body_text: &str, content_type: &str) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        let key = hex_prefix(hasher)?;
        let body_path = self.cache.join(format!("{key}.body"));
        let meta_path = self.cache.join(format!("{key}.meta.json"));
        std::fs::write(&body_path, body_text.as_bytes())
            .with_context(|| format!("seeding {}", body_path.display()))?;
        let mut body_hasher = Sha256::new();
        body_hasher.update(body_text.as_bytes());
        let meta = json!({
            "url": url,
            "method": method,
            "status": 200,
            "sha256": hex_prefix(body_hasher)?,
            "bytes": body_text.len(),
            "fetched_at": "2026-09-20T00:00:00Z",
            "content_type": content_type,
        });
        std::fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?)
            .with_context(|| format!("seeding {}", meta_path.display()))?;
        Ok(())
    }
}

/// Hex of the leading 16 bytes of a SHA-256 digest, the cache key form `net` uses.
fn hex_prefix(hasher: Sha256) -> Result<String> {
    let digest = hasher.finalize();
    let head = digest
        .get(..16)
        .context("sha256 digest shorter than its 16-byte prefix")?;
    Ok(head.iter().map(|byte| format!("{byte:02x}")).collect())
}

// -------------------------------------------------------------------------------------------------
// Case recording
// -------------------------------------------------------------------------------------------------

/// Asserts one case against its golden and records its digest for the source's aggregate.
fn case(cases: &mut Vec<(String, String)>, name: &str, value: &Value) -> Result<()> {
    common::assert_golden(name, value).with_context(|| format!("golden case `{name}`"))?;
    let digest = common::digest(value).with_context(|| format!("digesting case `{name}`"))?;
    cases.push((name.to_string(), digest));
    Ok(())
}

/// The per-source aggregate: every case name with the digest of its value, hashed as one document.
fn digest_all(source: &str, cases: &[(String, String)]) -> Result<()> {
    common::assert_golden(&format!("{source}__digest"), &serde_json::to_value(cases)?)
        .with_context(|| format!("aggregate digest for {source}"))
}

/// The fixture stem a golden case name is built from: `wi_roster_52649.html` → `wi_roster_52649`.
fn file_stem(name: &str) -> Result<String> {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .with_context(|| format!("fixture `{name}` has no file stem"))
}

// -------------------------------------------------------------------------------------------------
// athleticlive: the tenant/meet harvest artifact
// -------------------------------------------------------------------------------------------------

fn meet_row_json(row: &athleticlive::MeetRow) -> Value {
    json!({
        "tenant": row.tenant,
        "athleticlive_meet_id": row.athleticlive_meet_id,
        "athleticnet_meet_id": row.athleticnet_meet_id,
        "name": row.name,
        "city_state": row.city_state,
        "state_code": row.state_code,
        "start": row.start,
        "end": row.end,
        "has_results": row.has_results,
        "implausible_year": athleticlive::implausible_year(&row.start),
    })
}

#[tokio::test]
async fn athleticlive_harvest_parity() -> Result<()> {
    const SOURCE: &str = "athleticlive";
    let mut cases: Vec<(String, String)> = Vec::new();
    for path in common::fixtures(SOURCE)? {
        let file = common::file_name(&path)?;
        let stem = file_stem(&file)?;
        let body = common::fixture(SOURCE, &file)?;

        // The parse path: rows as published, then the canonical meets they mint.
        let rows = athleticlive::parse_meets_csv(&body)
            .with_context(|| format!("parsing {SOURCE}/{file}"))?;
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-rows"),
            &json!({
                "file": file,
                "rows": rows.iter().map(meet_row_json).collect::<Vec<Value>>(),
            }),
        )?;
        let meets = athleticlive::build_meets(&rows, OBSERVED_ON, "athleticlive_meets_csv");
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-meets"),
            &serde_json::to_value(&meets)?,
        )?;

        // The adapter's own entry point over the same bytes: journal, appended meets and report.
        let harness = Harness::new()?;
        let options = athleticlive::Options::for_input(path.display().to_string(), OBSERVED_ON);
        let report = athleticlive::collect(&harness.adapter_context(), &options)
            .await
            .with_context(|| format!("collecting {SOURCE}/{file}"))?;
        let written: Vec<CanonicalMeet> = harness.store.scan(Table::Meets)?;
        if written != meets {
            bail!(
                "`{SOURCE}::collect` wrote {} meets for {file}, the parse path produced {}",
                written.len(),
                meets.len()
            );
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-collect-report"),
            &serde_json::to_value(&report)?,
        )?;
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-collect-meets"),
            &serde_json::to_value(&written)?,
        )?;
    }
    digest_all(SOURCE, &cases)
}

// -------------------------------------------------------------------------------------------------
// athleticlive_athletes: the timer-published athlete index
// -------------------------------------------------------------------------------------------------

/// The rows an `athlete_list` response carries: `hits.hits[]._source`.
fn hits_from_response(body: &str) -> Result<Vec<AthleteHit>> {
    let value: Value = serde_json::from_str(body).context("an athlete_list response is JSON")?;
    let hits = value
        .pointer("/hits/hits")
        .and_then(Value::as_array)
        .context("the response has hits.hits")?;
    let sources: Vec<Value> = hits
        .iter()
        .map(|hit| hit.get("_source").cloned().unwrap_or(Value::Null))
        .collect();
    serde_json::from_value(Value::Array(sources)).context("the hits decode as athlete rows")
}

/// The meet the athlete capture belongs to: AthleticLIVE meet 73566 (Abilene Invitational), the
/// way the `athleticlive` adapter publishes it before this adapter selects it.
fn captured_meet() -> CanonicalMeet {
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Kansas),
        "Abilene Invitational",
        "2025-04-25",
        CompetitionLevel::Invitational,
    );
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "reddirt".to_string(),
        },
        "73566".to_string(),
    ));
    meet
}

fn athlete_hit_json(hit: &AthleteHit) -> Value {
    json!({
        "row_id": hit.athleticlive_row_id(),
        "i": hit.i,
        "name": hit.n,
        "grade_token": hit.y,
        "gender": hit.g,
        "meet_id": hit.meet_id(),
        "mi": hit.mi,
        "athletic_net_athlete_id": hit.athletic_net_athlete_id(),
        "ani": hit.ani,
        "team": hit.t.as_ref().map(|team| json!({
            "i": team.i,
            "n": team.n,
            "f": team.f,
            "ab": team.ab,
            "ani": team.ani,
            "xc": team.xc,
            "school_name": team.school_name(),
            "athleticlive_team_id": team.athleticlive_team_id(),
            "athletic_net_team_id": team.athletic_net_team_id(),
            "is_cross_country": team.is_cross_country(),
        })),
    })
}

fn meet_target_json(target: &MeetTarget) -> Value {
    json!({
        "athleticlive_meet_id": target.athleticlive_meet_id,
        "meet_id": target.meet_id,
        "tenant": target.tenant,
        "name": target.name,
        "state": target.state,
        "date": target.date,
    })
}

#[tokio::test]
async fn athleticlive_athletes_fixture_corpus_parity() -> Result<()> {
    const SOURCE: &str = "athleticlive_athletes";
    let mut cases: Vec<(String, String)> = Vec::new();
    for path in common::fixtures(SOURCE)? {
        let file = common::file_name(&path)?;
        let stem = file_stem(&file)?;
        let body = common::fixture(SOURCE, &file)?;

        let hits = hits_from_response(&body).with_context(|| format!("reading {SOURCE}/{file}"))?;
        if hits.is_empty() {
            bail!("{SOURCE}/{file} carries no rows to assert");
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-hits"),
            &json!({
                "file": file,
                "rows": hits.iter().map(athlete_hit_json).collect::<Vec<Value>>(),
            }),
        )?;

        // Target selection and the query body the adapter pages with.
        let selection =
            athleticlive_athletes::meet_targets(&[captured_meet()], &[UsJurisdiction::Kansas]);
        let targets: Vec<MeetTarget> = selection.targets;
        if targets.is_empty() {
            bail!("{SOURCE}/{file}: the captured meet is not selectable");
        }
        let ids: Vec<u64> = targets
            .iter()
            .map(|target| target.athleticlive_meet_id)
            .collect();
        for hit in &hits {
            match hit.meet_id() {
                Some(meet_id) if ids.contains(&meet_id) => {}
                other => {
                    bail!("{SOURCE}/{file}: row {other:?} is outside the selected meets {ids:?}")
                }
            }
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-targets"),
            &json!({
                "targets": targets.iter().map(meet_target_json).collect::<Vec<Value>>(),
                "skipped_implausible": selection.skipped_implausible,
                "query_from_0": athleticlive_athletes::batch_query(&ids, 0),
                "query_from_2000": athleticlive_athletes::batch_query(&ids, 2000),
            }),
        )?;

        // The rows as canonical entities, the way this adapter's `#[cfg(test)]` module drives it.
        let by_id: HashMap<u64, &MeetTarget> = targets
            .iter()
            .map(|target| (target.athleticlive_meet_id, target))
            .collect();
        let entities =
            athleticlive_athletes::build_entities(&hits, &by_id, OBSERVED_ON, SCHOOL_YEAR);
        if entities.rows != hits.len() {
            bail!(
                "{SOURCE}/{file}: {} of {} rows reached an entity pass",
                entities.rows,
                hits.len()
            );
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-entities"),
            &json!({
                "rows": entities.rows,
                "rows_with_grade": entities.rows_with_grade,
                "rows_with_athlete_id": entities.rows_with_athlete_id,
                "rows_with_team_id": entities.rows_with_team_id,
                "rows_without_school": entities.rows_without_school,
                "schools": entities.schools,
                "teams": entities.teams,
                "athletes": entities.athletes,
            }),
        )?;
    }
    digest_all(SOURCE, &cases)
}

// -------------------------------------------------------------------------------------------------
// milesplit: the per-state team index and graded roster HTML
// -------------------------------------------------------------------------------------------------

fn team_json(team: &milesplit::TeamRef) -> Value {
    json!({
        "id": team.id,
        "slug": team.slug,
        "url": team.url,
        "name": team.name,
        "city_state": team.city_state,
    })
}

fn meet_json(meet: &milesplit::MeetRef) -> Value {
    json!({
        "meet_id": meet.meet_id,
        "name": meet.name,
        "date": meet.date,
        "venue": meet.venue,
        "results_url": meet.results_url,
    })
}

fn roster_json(roster: &milesplit::Roster) -> Value {
    json!({
        "team": team_json(&roster.team),
        "athletes": roster.athletes.iter().map(|athlete| json!({
            "roster_name": athlete.roster_name,
            "name": athlete.name,
            "gender": athlete.gender,
            "grad_year": athlete.grad_year,
            "athlete_id": athlete.athlete_id,
            "profile_url": athlete.profile_url,
            "indoor": athlete.indoor,
            "outdoor": athlete.outdoor,
            "xc": athlete.xc,
            "sports": athlete.sports(),
        })).collect::<Vec<Value>>(),
    })
}

/// A roster capture's site prefix and team id: `wi_roster_52649.html` → `("wi", "52649")`.
///
/// Returns `None` for names this shape does not describe, so an unrecognised capture is reported
/// instead of being quietly skipped.
fn roster_fixture(name: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = name
        .strip_suffix(".html")?
        .split('_')
        .filter(|part| !part.is_empty())
        .collect();
    match (parts.first(), parts.get(1), parts.get(2), parts.len()) {
        // A three-part name is the bare form (`wi_roster_52649.html`); a fourth part is the slug the
        // site publishes after the id (`oh_roster_10002_mason.html`).
        (Some(site), Some(&"roster"), Some(team_id), 3 | 4) => {
            Some(((*site).to_string(), (*team_id).to_string()))
        }
        _ => None,
    }
}

#[tokio::test]
async fn milesplit_html_parity() -> Result<()> {
    const SOURCE: &str = "milesplit";
    let mut cases: Vec<(String, String)> = Vec::new();
    let mut indexes: HashMap<String, (String, Vec<milesplit::TeamRef>)> = HashMap::new();
    let mut rosters: Vec<(String, String, String, String, String)> = Vec::new();

    // Pass one: team indexes, keyed by the site prefix their file name carries.
    for path in common::fixtures(SOURCE)? {
        let file = common::file_name(&path)?;
        let stem = file_stem(&file)?;
        let body = common::fixture(SOURCE, &file)?;
        if let Some(site) = file.strip_suffix("_teams_index.html") {
            let teams = milesplit::parse_team_index(&body)
                .with_context(|| format!("parsing {SOURCE}/{file}"))?;
            if teams.is_empty() {
                bail!("{SOURCE}/{file} carries no teams to assert");
            }
            case(
                &mut cases,
                &format!("{SOURCE}__{stem}"),
                &json!({
                    "file": file,
                    "teams": teams.iter().map(team_json).collect::<Vec<Value>>(),
                }),
            )?;
            indexes.insert(site.to_string(), (file, teams));
        } else if let Some((site, team_id)) = roster_fixture(&file) {
            rosters.push((file, stem, body, site, team_id));
        } else if file.contains("_results_index") {
            let meets = milesplit::parse_meet_index(&body)
                .with_context(|| format!("parsing {SOURCE}/{file}"))?;
            if meets.is_empty() {
                bail!("{SOURCE}/{file} carries no meets to assert");
            }
            case(
                &mut cases,
                &format!("{SOURCE}__{stem}"),
                &json!({
                    "file": file,
                    "meets": meets.iter().map(meet_json).collect::<Vec<Value>>(),
                }),
            )?;
        } else if file.starts_with("oh_meet_") {
            // The per-meet captures: a meet's results page, its file-list page and a `/raw` body.
            // Their routes are asserted end to end by the adapter's own tests (`milesplit::tests`
            // parses the captures and maps their rows into canonical entities), so this harness —
            // whose case set is the index, roster and results-index routes — names them instead of
            // aborting the whole walk.
            continue;
        } else {
            bail!("uncovered {SOURCE} fixture `{file}`: parity_national.rs has no case for it");
        }
    }

    // Pass two: each roster resolves its team out of the matching index capture.
    for (file, stem, body, site_id, team_id) in rosters {
        let jurisdiction = UsJurisdiction::from_code(&site_id)
            .with_context(|| format!("{site_id} is not a USPS jurisdiction code"))?;
        let site = milesplit::Site::for_jurisdiction(jurisdiction);
        let (index_file, index_teams) = indexes.get(&site_id).with_context(|| {
            format!("no `{site_id}_teams_index.html` capture to resolve {file}'s team with")
        })?;
        let team = index_teams
            .iter()
            .find(|team| team.id == team_id)
            .with_context(|| format!("team {team_id} is not in the {index_file} capture"))?
            .clone();

        let roster = milesplit::parse_roster(&body, team.clone())
            .with_context(|| format!("parsing {SOURCE}/{file}"))?;
        if roster.athletes.is_empty() {
            bail!("{SOURCE}/{file} carries no athletes to assert");
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}"),
            &json!({ "file": file, "roster": roster_json(&roster) }),
        )?;

        let (school, athletes, teams) =
            milesplit::roster_entities(&roster, SCHOOL_YEAR, OBSERVED_ON, &site);
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-entities"),
            &json!({ "school": school, "athletes": athletes, "teams": teams }),
        )?;

        // The fetch entry points the census calls, over the same captures. The pairs must be equal
        // by construction — the fetcher serves the seeded bytes — so a drift in either direction is
        // a failure, and `requests == 0` proves nothing left the machine.
        let harness = Harness::new()?;
        let index_body = common::fixture(SOURCE, index_file)?;
        harness.seed(
            "GET",
            &site.teams_url(),
            &index_body,
            "text/html; charset=utf-8",
        )?;
        let fetched_teams =
            milesplit::fetch_team_index(&harness.fetcher, site, &FetchOptions::default())
                .await
                .with_context(|| format!("fetching {}", site.teams_url()))?;
        if fetched_teams != *index_teams {
            bail!(
                "fetch_team_index returned {} teams, parse_team_index returned {} for {index_file}",
                fetched_teams.len(),
                index_teams.len()
            );
        }

        let roster_url = format!("{}/roster", team.url);
        harness.seed("GET", &roster_url, &body, "text/html; charset=utf-8")?;
        let fetched_roster =
            milesplit::fetch_roster(&harness.fetcher, &team, &FetchOptions::default())
                .await
                .with_context(|| format!("fetching {roster_url}"))?;
        if fetched_roster != roster {
            bail!(
                "fetch_roster returned {} athletes, parse_roster returned {} for {file}",
                fetched_roster.athletes.len(),
                roster.athletes.len()
            );
        }
        let stats = harness.fetcher.stats().await;
        if stats.requests != 0 {
            bail!(
                "{SOURCE}/{file}: the seeded cache missed ({} live requests) — a real response, not a capture, was read",
                stats.requests
            );
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-fetched"),
            &json!({
                "file": file,
                "cache_hits": stats.cache_hits,
                "teams": fetched_teams.iter().map(team_json).collect::<Vec<Value>>(),
                "roster": roster_json(&fetched_roster),
            }),
        )?;
    }
    digest_all(SOURCE, &cases)
}

// -------------------------------------------------------------------------------------------------
// coach_contacts: the contact CSV
// -------------------------------------------------------------------------------------------------

#[tokio::test]
async fn coach_contacts_csv_parity() -> Result<()> {
    const SOURCE: &str = "coach_contacts";
    let mut cases: Vec<(String, String)> = Vec::new();
    let fixtures = common::fixtures_dir()?;

    // The contact captures sit in the fixtures root beside the per-source directories; only the
    // `coach_contacts*` files belong to this source.
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in
        std::fs::read_dir(&fixtures).with_context(|| format!("listing {}", fixtures.display()))?
    {
        let path = entry
            .with_context(|| format!("reading an entry of {}", fixtures.display()))?
            .path();
        let name = common::file_name(&path)?;
        if path.is_file() && name.starts_with(SOURCE) {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        bail!("no {SOURCE}* captures under {}", fixtures.display());
    }

    for path in paths {
        let file = common::file_name(&path)?;
        let stem = file_stem(&file)?;
        let body = common::fixture("", &file)?;

        // Every row of the capture, including the rows whose role imports nobody.
        let mut reader = csv::Reader::from_reader(body.as_bytes());
        let parsed: Vec<coach_contacts::CoachContactRow> = reader
            .deserialize()
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("decoding {SOURCE}/{file}"))?;
        if parsed.is_empty() {
            bail!("{SOURCE}/{file} carries no rows to assert");
        }
        let mut rows = Vec::with_capacity(parsed.len());
        let mut expected_schools: BTreeSet<String> = BTreeSet::new();
        let mut expected_coaches: BTreeSet<String> = BTreeSet::new();
        for (index, row) in parsed.iter().enumerate() {
            let state = UsJurisdiction::from_code(row.state.trim()).with_context(|| {
                format!(
                    "{SOURCE}/{file} row {}: state {:?}",
                    index.saturating_add(2),
                    row.state
                )
            })?;
            let entities = coach_contacts::row_entities(row, state, OBSERVED_ON)
                .with_context(|| format!("{SOURCE}/{file} row {}", index.saturating_add(2)))?;
            expected_schools.insert(entities.school.id.as_str().to_string());
            for coach in &entities.coaches {
                expected_coaches.insert(coach.id.as_str().to_string());
            }
            rows.push(json!({
                "row": index.saturating_add(2),
                "school": entities.school,
                "coaches": entities.coaches,
            }));
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-rows"),
            &json!({ "file": file, "rows": rows }),
        )?;

        // The import entry point over the same capture, into a scratch store.
        let harness = Harness::new()?;
        let report = coach_contacts::import_csv(&harness.store, &path, OBSERVED_ON)
            .with_context(|| format!("importing {SOURCE}/{file}"))?;
        let schools: Vec<census_domain::model::CanonicalSchool> =
            harness.store.scan(Table::Schools)?;
        let coaches: Vec<census_domain::model::CanonicalCoach> =
            harness.store.scan(Table::Coaches)?;
        if schools.len() != expected_schools.len() || coaches.len() != expected_coaches.len() {
            bail!(
                "{SOURCE}/{file}: import_csv wrote {} schools and {} coaches, the row pass produced {} and {}",
                schools.len(),
                coaches.len(),
                expected_schools.len(),
                expected_coaches.len()
            );
        }
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-import-report"),
            &serde_json::to_value(&report)?,
        )?;
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-import-schools"),
            &serde_json::to_value(&schools)?,
        )?;
        case(
            &mut cases,
            &format!("{SOURCE}__{stem}-import-coaches"),
            &serde_json::to_value(&coaches)?,
        )?;
    }
    digest_all(SOURCE, &cases)
}

// -------------------------------------------------------------------------------------------------
// athleticnet: the bio surface, replayed from the module's own inline captures
// -------------------------------------------------------------------------------------------------

/// The registry the adapter's own tests read (comments, blank lines, a repeat, a per-line state and
/// a bare id the default fills in).
const REGISTRY: &str = "# season 2026\n28127170,AK\n\n26631105\n28127170,AK\n";

/// Registries the adapter refuses rather than guesses about: two candidate states, a state that is
/// not a postal code, a line that is not an id, and a third column.
const REFUSED_REGISTRIES: [(&str, &[UsJurisdiction]); 4] = [
    (
        "28127170\n",
        &[UsJurisdiction::Wisconsin, UsJurisdiction::Alaska],
    ),
    ("28127170,Alaska\n", &[]),
    ("natalia\n", &[]),
    ("28127170,AK,extra\n", &[]),
];

/// The published mark tokens the adapter's own tests cover: auto-timed, hand-timed, qualifier
/// suffix, imperial field mark, metric field mark, points, and the no-mark words.
const MARK_TOKENS: [(EventKind, &str); 11] = [
    (EventKind::Track800m, "1:17.80a"),
    (EventKind::Track3200m, "9:41.23"),
    (EventKind::Track100m, "11.32q"),
    (EventKind::LongJump, "5-04.25"),
    (EventKind::ShotPut, "12.34m"),
    (EventKind::Decathlon, "3,456"),
    (EventKind::Track1600m, "DNS"),
    (EventKind::Track1600m, "ND"),
    (EventKind::Track1600m, "FOUL"),
    (EventKind::Track1600m, "X"),
    (EventKind::Track1600m, ""),
];

/// A track-payload capture: a string place, an event id, a result date and a null `resultsXC`.
const BIO_TRACK_FIELD: &str = r#"{
  "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
              "Gender": "F", "SchoolID": 13850},
  "grades": {"13850_2026": 11},
  "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
  "allSeasons": [
     {"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Indoor"},
     {"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Outdoor"}
  ],
  "eventsTF": [{"IDEvent": 20, "Event": "200 Meters"}],
  "meets": {"589334": {"MeetName": "SOHI Invite", "EndDate": "2025-05-03T00:00:00"}},
  "resultsTF": [
    {"IDResult": 1, "Result": "26.10a", "FAT": 1, "Place": "3", "Round": "F",
     "Wind": 1.2, "Division": "Varsity", "SchoolID": 13850, "EventID": 20,
     "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2025-05-02T00:00:00"},
    {"IDResult": 2, "Result": "DNS", "FAT": 0, "Place": "", "Round": "P",
     "Division": "Varsity", "SchoolID": 13850, "EventID": 20,
     "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2025-05-02T00:00:00"}
  ],
  "resultsXC": null
}"#;

/// A cross-country capture: a numeric place, a course distance, no result date and no `eventsTF`.
const BIO_CROSS_COUNTRY: &str = r#"{
  "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
              "Gender": "F", "SchoolID": 13850},
  "grades": {"13850_2026": 11},
  "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
  "allSeasons": [],
  "meets": {"223703": {"MeetName": "Chandler Invitational", "EndDate": "2025-09-02T00:00:00"}},
  "resultsXC": [
    {"IDResult": 47122798, "Result": "25:31.2", "Place": 68, "Division": "Varsity",
     "SchoolID": 13850, "MeetID": 223703, "SeasonID": 2025, "Distance": 5000}
  ]
}"#;

fn bio_json(bio: &athleticnet::Bio) -> Value {
    json!({
        "athlete": {
            "id": bio.athlete.id,
            "name": bio.athlete.name(),
            "first_name": bio.athlete.first_name,
            "last_name": bio.athlete.last_name,
            "gender": bio.athlete.gender,
            "school_id": bio.athlete.school_id,
        },
        "grades": bio.grades,
        "teams": bio.teams.iter().map(|(school_id, team)| json!({
            "school_id": school_id,
            "school_name": team.school_name,
        })).collect::<Vec<Value>>(),
        "seasons": bio.seasons.iter().map(|season| json!({
            "school_id": season.school_id,
            "season_id": season.season_id,
            "display": season.display,
        })).collect::<Vec<Value>>(),
        "events": bio.events.as_ref().map(|events| events.iter().map(|event| json!({
            "id": event.id,
            "label": event.label,
        })).collect::<Vec<Value>>()),
        "results_tf": bio.results_tf.as_ref().map(|rows| rows.iter().map(|row| json!({
            "id": row.id,
            "result": row.result,
            "fat": row.fat,
            "place": row.place,
            "round": row.round,
            "wind": row.wind,
            "division": row.division,
            "school_id": row.school_id,
            "event_id": row.event_id,
            "meet_id": row.meet_id,
            "season_id": row.season_id,
            "result_date": row.result_date,
        })).collect::<Vec<Value>>()),
        "results_xc": bio.results_xc.as_ref().map(|rows| rows.iter().map(|row| json!({
            "id": row.id,
            "result": row.result,
            "place": row.place,
            "division": row.division,
            "school_id": row.school_id,
            "meet_id": row.meet_id,
            "season_id": row.season_id,
            "distance": row.distance,
        })).collect::<Vec<Value>>()),
        "meets": bio.meets.iter().map(|(meet_id, meet)| json!({
            "meet_id": meet_id,
            "name": meet.name,
            "end_date": meet.end_date,
        })).collect::<Vec<Value>>(),
    })
}

#[tokio::test]
async fn athleticnet_inline_capture_parity() -> Result<()> {
    const SOURCE: &str = "athleticnet";
    let mut cases: Vec<(String, String)> = Vec::new();

    // The registry: a per-line state wins, the default fills a bare id, a repeat reads once.
    let targets = athleticnet::parse_targets(REGISTRY, &[UsJurisdiction::Wisconsin])?;
    case(
        &mut cases,
        &format!("{SOURCE}__registry-targets"),
        &json!({
            "registry": REGISTRY,
            "targets": targets.iter().map(|target| json!({
                "athlete_id": target.athlete_id,
                "state": target.state,
            })).collect::<Vec<Value>>(),
        }),
    )?;

    // The refusals, message and all: an ambiguous or malformed registry is an error, not a guess.
    let mut refusals: Vec<Value> = Vec::new();
    for (registry, states) in REFUSED_REGISTRIES {
        let states: Vec<UsJurisdiction> = states.to_vec();
        match athleticnet::parse_targets(registry, &states) {
            Ok(parsed) => bail!(
                "registry `{registry}` was accepted with {} targets where the adapter refuses",
                parsed.len()
            ),
            Err(error) => refusals.push(json!({
                "registry": registry,
                "states": states,
                "refusal": error.to_string(),
            })),
        }
    }
    case(
        &mut cases,
        &format!("{SOURCE}__registry-refusals"),
        &json!({ "refusals": refusals }),
    )?;

    // The published mark tokens: what each one denotes and whether it is automatic.
    let marks: Vec<Value> = MARK_TOKENS
        .iter()
        .map(|(kind, published)| {
            json!({
                "event": format!("{kind:?}"),
                "published": published,
                "mark": athleticnet::parse_mark(kind, published),
            })
        })
        .collect();
    case(
        &mut cases,
        &format!("{SOURCE}__marks"),
        &json!({ "marks": marks }),
    )?;

    // The two published bio shapes the adapter decodes.
    let track: athleticnet::Bio =
        serde_json::from_str(BIO_TRACK_FIELD).context("the track payload decodes")?;
    let track_rows = track
        .results_tf
        .as_ref()
        .context("the track payload carries resultsTF")?;
    if track_rows.first().and_then(|row| row.place.as_deref()) != Some("3") {
        bail!("a string place no longer reads as a place");
    }
    if track_rows
        .get(1)
        .and_then(|row| row.place.as_deref())
        .is_some()
    {
        bail!("an empty place reads as a place again");
    }
    if track.results_xc.is_some() {
        bail!("a null resultsXC no longer reads as no rows");
    }
    case(
        &mut cases,
        &format!("{SOURCE}__bio-track-field"),
        &bio_json(&track),
    )?;

    let cross_country: athleticnet::Bio =
        serde_json::from_str(BIO_CROSS_COUNTRY).context("the cross-country payload decodes")?;
    let xc_rows = cross_country
        .results_xc
        .as_ref()
        .context("the cross-country payload carries resultsXC")?;
    if xc_rows.first().and_then(|row| row.place.as_deref()) != Some("68") {
        bail!("a numeric place no longer reads as a place");
    }
    if xc_rows.first().and_then(|row| row.distance) != Some(5000) {
        bail!("the published course distance no longer reads");
    }
    case(
        &mut cases,
        &format!("{SOURCE}__bio-cross-country"),
        &bio_json(&cross_country),
    )?;

    digest_all(SOURCE, &cases)
}
