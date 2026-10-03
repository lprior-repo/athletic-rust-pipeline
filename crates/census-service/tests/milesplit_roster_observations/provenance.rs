use super::{collect_options, milesplit, replay, seed_cache, WI_ROSTER_FIXTURE, WI_TEAMS_FIXTURE};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::CrawlError;
use census_domain::model::{CanonicalAthlete, CanonicalSchool, SourceObservation};
use census_domain::UsJurisdiction;
use census_service::census;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[test]
fn an_authentic_ownerless_fragment_remains_parseable_but_cannot_commit_an_acquired_roster(
) -> super::TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let team = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?
                .into_iter()
                .next()
                .ok_or("historical index lists no teams")?;
            let parsed = milesplit::parse_roster(WI_ROSTER_FIXTURE, team.clone())?;
            check!(eq; parsed.roster().ok_or("fragment is not readable")?.athletes.len(),
    25);
            let url = format!("{}/roster", team.url);
            seed_cache(&store.http_cache_dir(), &url, WI_ROSTER_FIXTURE)?;
            let fetcher = Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?;
            let capture = fetcher.get(&url, &FetchOptions::default()).await?;
            check!(eq; capture.response_url, None);
            check!(eq; capture.body, WI_ROSTER_FIXTURE.as_bytes());
            check!(eq; capture.bytes, WI_ROSTER_FIXTURE.len());
            check!(eq; capture.content_digest,
    format!("{:x}", Sha256::digest(WI_ROSTER_FIXTURE.as_bytes())));
            let outcome = milesplit::fetch_roster(&fetcher, &team, &FetchOptions::default()).await;
            check!(matches!(outcome, Err(CrawlError::Schema { url, .. }) if url == team.url));
            let options = collect_options();
            let before = replay::digest(&store)?;
            let progress = census::collect_state_rosters(
                &fetcher,
                &store,
                std::slice::from_ref(&team),
                &options,
                UsJurisdiction::Wisconsin,
            )
            .await?;
            check!(eq; progress.rosters_committed, 0);
            check!(eq; progress.rosters_remaining, 1);
            check!(eq; progress.athletes, 0);
            check!(eq; progress.class_of_2027, 0);
            check!(eq; progress.errors.len(), 1);
            check!(eq; replay::digest(&store)?, before);
            drop(store);
            let reopened = Store::open(dir.path())?;
            let phase = census::rosters_phase(
                UsJurisdiction::Wisconsin,
                options.school_year,
                options.revision,
            );
            check!(eq; reopened.journal_keys(&phase)?,
    std::collections::HashSet::<String>::new());
            check!(eq; reopened
        .scan::<CanonicalSchool>(Table::Schools)?,
    Vec::<CanonicalSchool>::new());
            check!(eq; reopened
        .scan::<CanonicalAthlete>(Table::Athletes)?,
    Vec::<CanonicalAthlete>::new());
            check!(eq; reopened
        .scan::<SourceObservation>(Table::SourceObservations)?,
    Vec::<SourceObservation>::new());
            check!(eq; replay::digest(&reopened)?, before);
            let retained = fetcher.get(&capture.url, &FetchOptions::default()).await?;
            check!(eq; retained.response_url, None);
            check!(eq; retained.body, capture.body);
            check!(eq; retained.content_digest, capture.content_digest);
            Ok(())
        })
}
