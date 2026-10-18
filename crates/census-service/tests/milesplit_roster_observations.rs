#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "common/capture_cache.rs"]
mod capture_cache;

use census_crawl::milesplit::{self, Site};
use census_crawl::net::Fetcher;
use census_domain::model::{
    CanonicalSchool, SchoolYear, SourceAthleteObservation, SourceNamespace, SourceObservation,
};
use census_domain::UsJurisdiction;
use census_service::census::{self, CollectOptions};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "milesplit_roster_observations/locator.rs"]
mod locator;
#[path = "milesplit_roster_observations/outcomes.rs"]
mod outcomes;
#[path = "milesplit_roster_observations/projection.rs"]
mod projection;
#[path = "milesplit_roster_observations/provenance.rs"]
mod provenance;
#[path = "milesplit_roster_observations/replay.rs"]
mod replay;

const OBSERVED_ON: &str = "2026-09-20";
const CAPTURED_AT: &str = "2026-09-20T00:00:00Z";
const WI_TEAMS_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_teams_index.html");
const WI_ROSTER_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_roster_52649.html");
const ROSTER_URL: &str = "https://wi.milesplit.com/teams/52649-abbotsford/roster";

fn synthetic_owned_roster_document(team: &milesplit::TeamRef, fragment: &str) -> String {
    format!(
        "<!doctype html><html data-fixture=\"synthetic-owned-roster\"><head>\
         <link rel=\"canonical\" href=\"{}/roster\"></head><body>{fragment}</body></html>",
        team.url
    )
}

async fn configure_inventory(
    store: &Store,
    fetcher: &Fetcher,
    team: &milesplit::TeamRef,
) -> TestResult {
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let synthetic = format!(
        "<html data-fixture=\"synthetic-single-team-inventory\"><head>\
         <meta name=\"application-name\" content=\"MileSplit\"></head><body><table>\
         <tr><td><a href=\"{}\">{}</a></td><td>{}</td></tr></table></body></html>",
        team.url, team.name, team.city_state
    );
    seed_cache(&store.http_cache_dir(), &site.teams_url(), &synthetic)?;
    let teams =
        census::collect_state_teams(fetcher, store, UsJurisdiction::Wisconsin, false).await?;
    check!(eq; teams, vec![team.clone()]);
    Ok(())
}

fn seed_cache(cache: &Path, url: &str, body: &str) -> TestResult {
    capture_cache::seed_response(
        cache,
        url,
        body.as_bytes(),
        CAPTURED_AT,
        capture_cache::Metadata {
            content_type: Some("text/html; charset=utf-8"),
            response_url: capture_cache::ResponseUrl::Null,
            ..capture_cache::Metadata::default()
        },
    )?;
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
            let teams = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?.teams;
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
            )?
            .with_offline(true);
            configure_inventory(&store, &fetcher, &first).await?;
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
                .map(|member| {
                    let athlete = member.athlete();
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
            let captured_digest = format!("{:x}", Sha256::digest(capture.body.as_slice()));
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
                check!(eq; observation.observed_on, CAPTURED_AT,
    "domain row preserves the capture's RFC3339 instant, not a restamped caller date");
                check!(eq; observation.observed_grade, None);
                let row = locator::parse(&observation.source_row_key)?;
                check!(eq; row.resource_url, ROSTER_URL);
                check!(eq; row.capture_sha256, captured_digest.as_str());
                locator::assert_span_contains_profile(&row, capture.body.as_slice(), profile_url)?;
            }

            let mut ordinals = Vec::new();
            let mut spans = Vec::new();
            for id in published.keys() {
                let observation = filed.get(id.as_str()).ok_or("missing observation")?;
                let row = locator::parse(&observation.source_row_key)?;
                check!(row.ordinal < 25);
                ordinals.push(row.ordinal);
                spans.push((row.byte_offset, row.byte_length));
            }
            ordinals.sort();
            ordinals.dedup();
            check!(eq; ordinals.len(), published.len(),
    "each published athlete has a distinct row ordinal");
            spans.sort();
            spans.dedup();
            check!(eq; spans.len(), published.len(),
    "each published athlete has a distinct physical byte span");

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
