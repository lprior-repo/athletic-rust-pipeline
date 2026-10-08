#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::milesplit::{self, Site};
use census_crawl::net::Fetcher;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, SchoolYear, SourceAthleteObservation, SourceNamespace,
    SourceObservation,
};
use census_domain::UsJurisdiction;
use census_service::census::{self, CollectOptions};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "milesplit_roster_observations/projection.rs"]
mod projection;
#[path = "milesplit_roster_observations/provenance.rs"]
mod provenance;
#[path = "milesplit_roster_observations/replay.rs"]
mod replay;

const OBSERVED_ON: &str = "2026-09-20";
const WI_TEAMS_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_teams_index.html");
const WI_ROSTER_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_roster_52649.html");

fn synthetic_owned_roster_document(team: &milesplit::TeamRef, fragment: &str) -> String {
    format!(
        "<!doctype html><html data-fixture=\"synthetic-owned-roster\"><head>\
         <link rel=\"canonical\" href=\"{}/roster\"></head><body>{fragment}</body></html>",
        team.url
    )
}

fn seed_cache(cache: &Path, url: &str, body: &str) -> TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    let meta = serde_json::json!({
        "url": url,
        "response_url": null,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "text/html; charset=utf-8",
    });
    std::fs::write(cache.join(format!("{key}.meta.json")), meta.to_string())?;
    Ok(())
}

#[test]
fn a_synthetic_owned_roster_pass_files_an_observation_per_published_athlete() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
            seed_cache(&store.http_cache_dir(), &site.teams_url(), WI_TEAMS_FIXTURE)?;
            let teams = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?;
            let first = teams.first().ok_or("index fixture lists no teams")?.clone();
            let synthetic_body = synthetic_owned_roster_document(&first, WI_ROSTER_FIXTURE);
            seed_cache(
                &store.http_cache_dir(),
                &format!("{}/roster", first.url),
                &synthetic_body,
            )?;
            let fetcher = Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?;
            let capture = fetcher
                .get(
                    &format!("{}/roster", first.url),
                    &census_crawl::net::FetchOptions::default(),
                )
                .await?;
            check!(eq; capture.response_url, None);
            check!(eq; capture.body, synthetic_body.as_bytes());
            let options = collect_options();
            let progress = census::collect_state_rosters(
                &fetcher,
                &store,
                std::slice::from_ref(&first),
                &options,
                UsJurisdiction::Wisconsin,
            )
            .await?;
            check!(eq; progress.errors, Vec::<String>::new());
            check!(eq; progress.rosters_committed, 1,
    "the pass read the one team it was given");

            let parsed = milesplit::parse_roster(WI_ROSTER_FIXTURE, first.clone())?;
            let roster = parsed
                .roster()
                .ok_or("roster fixture carries no readable athletes")?;
            let published: BTreeMap<String, (&str, &str)> = roster
                .athletes
                .iter()
                .map(|athlete| {
                    (
                        athlete.athlete_id.clone(),
                        (athlete.name.as_str(), athlete.profile_url.as_str()),
                    )
                })
                .collect();
            check!(eq; published.len(),
    roster.athletes.len(),
    "the capture's athlete ids are distinct");

            let mut observations = Vec::new();
            store.snapshot().for_each_observation(
                Table::SourceObservations,
                |row: SourceObservation| {
                    observations.push(row);
                    Ok(())
                },
            )?;
            let filed: BTreeMap<&str, &SourceAthleteObservation> = observations
                .iter()
                .filter_map(|row| match row {
                    SourceObservation::Athlete(athlete) => {
                        Some((athlete.source_athlete_id.as_str(), athlete))
                    }
                    SourceObservation::School(_) => None,
                })
                .collect();
            check!(eq; filed.keys().copied().collect::<Vec<_>>(),
    published.keys().map(String::as_str).collect::<Vec<_>>(),
    "one observation per athlete the roster published, keyed by MileSplit's own id");

            let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
            check!(eq; schools.len(), 1, "one indexed roster, one school");
            for (id, (name, profile_url)) in &published {
                let observation = filed
                    .get(id.as_str())
                    .ok_or("published athlete has no filed observation")?;
                check!(eq; observation.namespace, SourceNamespace::MilesplitAthlete);
                check!(eq; observation.observed_name.as_str(),
        *name,
        "the name the roster page spelled");
                check!(eq; observation.observed_school.as_deref(),
        Some(schools[0].name.as_str()),
        "the school the source placed the athlete at");
                check!(eq; observation.profile_url.as_deref(),
        Some(*profile_url),
        "the profile page the id was read from");
                check!(eq; observation.observed_on, OBSERVED_ON);
                check!(eq; observation.observed_grade, None);
                check!(eq; observation.source_row_key, *profile_url);
            }

            projection::assert_published(&store)?;
            projection::assert_capture(&store, &options, &synthetic_body)?;
            let before = replay::digest(&store)?;
            drop(store);
            replay::assert_reopened_replay(
                dir.path(),
                &fetcher,
                &first,
                &options,
                &before,
                &synthetic_body,
            )
            .await?;
            Ok(())
        })
}

fn collect_options() -> CollectOptions {
    CollectOptions {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        limit_per_state: None,
        concurrency: 1,
        state_concurrency: 1,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: OBSERVED_ON.to_string(),
        revision: std::num::NonZeroU32::MIN,
    }
}

#[test]
fn partial_and_quarantined_rosters_keep_their_reasons_and_still_owe_a_walk() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let partial = WI_ROSTER_FIXTURE.replacen(
                "column-grad-year\">2027",
                "column-grad-year\">invalid",
                1,
            );
            for (body, accepted) in [
                (partial.as_str(), 24),
                ("<html>unrecognized page</html>", 0),
            ] {
                let dir = tempfile::tempdir()?;
                let store = Store::open(dir.path())?;
                let team = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?
                    .into_iter()
                    .next()
                    .ok_or("index fixture lists no teams")?;
                let synthetic_body = synthetic_owned_roster_document(&team, body);
                seed_cache(
                    &store.http_cache_dir(),
                    &format!("{}/roster", team.url),
                    &synthetic_body,
                )?;
                let fetcher = Fetcher::new(
                    store.http_cache_dir(),
                    None,
                    Duration::from_millis(1),
                    std::collections::HashMap::new(),
                    Vec::new(),
                )?;
                let options = collect_options();
                let first = census::collect_state_rosters(
                    &fetcher,
                    &store,
                    std::slice::from_ref(&team),
                    &options,
                    UsJurisdiction::Wisconsin,
                )
                .await?;
                check!(eq; first.rosters_committed, 0);
                check!(eq; first.rosters_skipped, 1);
                check!(eq; first.rosters_remaining, 0);
                check!(eq;
                    first.rosters_committed + first.rosters_skipped + first.rosters_remaining,
                    first.rosters_total,
                    "a partially parsed roster keeps its valid rows and still owes a walk");
                check!(eq; first.athletes, accepted);
                check!(eq; first.errors.len(), 1);
                let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
                check!(eq; athletes.len(), accepted);
                let observations: Vec<SourceObservation> = store.scan(Table::SourceObservations)?;
                check!(eq; observations
            .iter()
            .filter(|row| matches!(row, SourceObservation::Athlete(_)))
            .count(),
        accepted);
                let expected_observations = accepted + usize::from(accepted != 0);
                check!(eq; observations.len(), expected_observations);
                let before = replay::digest(&store)?;
                drop(store);
                let reopened = Store::open(dir.path())?;
                let resumed = census::collect_state_rosters(
                    &fetcher,
                    &reopened,
                    std::slice::from_ref(&team),
                    &options,
                    UsJurisdiction::Wisconsin,
                )
                .await?;
                check!(eq; resumed.errors, first.errors);
                check!(eq; resumed.rosters_committed, 0);
                check!(eq; resumed.rosters_remaining, 0);
                check!(eq; resumed.rosters_skipped, 1);
                check!(eq; resumed.athletes, accepted);
                let persisted: Vec<SourceObservation> = reopened.scan(Table::SourceObservations)?;
                check!(eq; persisted.len(), expected_observations);
                check!(eq; replay::digest(&reopened)?, before);
                let stats = fetcher.stats().await;
                check!(eq; stats.physical_requests(), 0);
                check!(eq;
                    stats.cache_hits,
                    2,
                    "a partial or quarantined roster is walked again, from cache, because its journal cannot certify exhaustion");
            }
            Ok(())
        })
}

#[test]
fn a_refused_roster_is_retained_without_rows_or_a_refetch() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let mut team = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?
                .into_iter()
                .next()
                .ok_or("index fixture lists no teams")?;
            team.city_state = "Washington, DC".to_string();
            let synthetic_body = synthetic_owned_roster_document(&team, WI_ROSTER_FIXTURE);
            seed_cache(
                &store.http_cache_dir(),
                &format!("{}/roster", team.url),
                &synthetic_body,
            )?;
            let fetcher = Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?;
            let options = collect_options();
            let first = census::collect_state_rosters(
                &fetcher,
                &store,
                std::slice::from_ref(&team),
                &options,
                UsJurisdiction::Wisconsin,
            )
            .await?;
            check!(eq; first.rosters_committed, 0);
            check!(eq; first.rosters_skipped, 1);
            check!(eq; first.rosters_remaining, 0);
            check!(eq;
                first.rosters_committed + first.rosters_skipped + first.rosters_remaining,
                first.rosters_total,
                "a refused roster is an attempted team, not an owed fetch");
            check!(eq; first.athletes, 0);
            check!(eq; first.errors.len(), 1);
            let report = first.errors.first().ok_or("the refusal is reported")?;
            check!(
                report.contains("refusal=schema mismatch"),
                "the refusal keeps the source's own words"
            );
            check!(
                report.contains("requested team location conflicts with its source jurisdiction"),
                "the refusal names the conflicting jurisdiction"
            );
            let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
            check!(eq; athletes.len(), 0);
            let before = replay::digest(&store)?;
            drop(store);
            let reopened = Store::open(dir.path())?;
            let resumed = census::collect_state_rosters(
                &fetcher,
                &reopened,
                std::slice::from_ref(&team),
                &options,
                UsJurisdiction::Wisconsin,
            )
            .await?;
            check!(eq; resumed.errors, first.errors);
            check!(eq; resumed.rosters_committed, 0);
            check!(eq; resumed.rosters_skipped, 1);
            check!(eq; resumed.rosters_remaining, 0);
            check!(eq; replay::digest(&reopened)?, before);
            let stats = fetcher.stats().await;
            check!(eq; stats.physical_requests(), 0);
            check!(eq; stats.cache_hits, 1);
            Ok(())
        })
}
