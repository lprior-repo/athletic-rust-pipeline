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

const OBSERVED_ON: &str = "2026-09-20";
const WI_TEAMS_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_teams_index.html");
const WI_ROSTER_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_roster_52649.html");

fn seed_cache(cache: &Path, url: &str, body: &str) {
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
    std::fs::create_dir_all(cache).expect("cache dir");
    std::fs::write(cache.join(format!("{key}.body")), body).expect("cache body");
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "text/html; charset=utf-8",
    });
    std::fs::write(cache.join(format!("{key}.meta.json")), meta.to_string()).expect("cache meta");
}

#[tokio::test]
async fn a_roster_pass_files_an_observation_per_published_athlete() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("opening the store");
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    seed_cache(&store.http_cache_dir(), &site.teams_url(), WI_TEAMS_FIXTURE);
    let teams = milesplit::parse_team_index(WI_TEAMS_FIXTURE).expect("the index fixture parses");
    let first = teams
        .first()
        .expect("the index fixture lists teams")
        .clone();
    seed_cache(
        &store.http_cache_dir(),
        &format!("{}/roster", first.url),
        WI_ROSTER_FIXTURE,
    );
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("building the fetcher");
    let options = collect_options();
    let progress = census::collect_state_rosters(
        &fetcher,
        &store,
        std::slice::from_ref(&first),
        &options,
        UsJurisdiction::Wisconsin,
    )
    .await
    .expect("the roster pass completes");
    assert_eq!(progress.errors, Vec::<String>::new());
    assert_eq!(
        progress.rosters_committed, 1,
        "the pass read the one team it was given"
    );

    let parsed = milesplit::parse_roster(WI_ROSTER_FIXTURE, first.clone())
        .expect("the roster fixture parses");
    let roster = parsed.roster().expect("the roster fixture has readable athletes");
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
    assert_eq!(
        published.len(),
        roster.athletes.len(),
        "the capture's athlete ids are distinct"
    );

    let observations: Vec<SourceObservation> = store
        .scan(Table::SourceObservations)
        .expect("observation log");
    let filed: BTreeMap<&str, &SourceAthleteObservation> = observations
        .iter()
        .filter_map(|row| match row {
            SourceObservation::Athlete(athlete) => {
                Some((athlete.source_athlete_id.as_str(), athlete))
            }
            SourceObservation::School(_) => None,
        })
        .collect();
    assert_eq!(
        filed.keys().copied().collect::<Vec<_>>(),
        published.keys().map(String::as_str).collect::<Vec<_>>(),
        "one observation per athlete the roster published, keyed by MileSplit's own id"
    );

    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools).expect("schools read");
    assert_eq!(schools.len(), 1, "one indexed roster, one school");
    for (id, (name, profile_url)) in &published {
        let observation = filed
            .get(id.as_str())
            .expect("every published athlete id is filed");
        assert_eq!(observation.namespace, SourceNamespace::MilesplitAthlete);
        assert_eq!(
            observation.observed_name.as_str(),
            *name,
            "the name the roster page spelled"
        );
        assert_eq!(
            observation.observed_school.as_deref(),
            Some(schools[0].name.as_str()),
            "the school the source placed the athlete at"
        );
        assert_eq!(
            observation.profile_url.as_deref(),
            Some(*profile_url),
            "the profile page the id was read from"
        );
        assert_eq!(observation.observed_on, OBSERVED_ON);
        assert!(
            observation.observed_grade.is_some(),
            "the roster publishes a class for every athlete it lists"
        );
        assert!(
            !observation.source_row_key.is_empty(),
            "the row states where it was read"
        );
    }

    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes read");
    assert_eq!(athletes.len(), published.len());
    for athlete in &athletes {
        let identity = &athlete.source;
        assert_eq!(identity.namespace, SourceNamespace::MilesplitAthlete);
        assert!(
            filed.contains_key(identity.id.as_str()),
            "the observation and the canonical row name the same provider object: {}",
            identity.id
        );
    }

    let sample = roster.athletes.first().expect("the fixture lists athletes");
    let observation = filed
        .get(sample.athlete_id.as_str())
        .expect("the fixture's first athlete is filed under the provider's own id");
    assert_eq!(observation.observed_name, sample.name);
    assert_eq!(observation.observed_school.as_deref(), Some("Abbotsford"));
    assert_eq!(
        observation.profile_url.as_deref(),
        Some(sample.profile_url.as_str())
    );
}

fn collect_options() -> CollectOptions {
    CollectOptions {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        limit_per_state: None,
        concurrency: 1,
        state_concurrency: 1,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("valid year"),
        observed_on: OBSERVED_ON.to_string(),
        revision: std::num::NonZeroU32::MIN,
    }
}

#[tokio::test]
async fn partial_and_quarantined_rosters_remain_incomplete_after_reopening() {
    let partial = WI_ROSTER_FIXTURE.replacen(
        "column-grad-year\">2027", "column-grad-year\">invalid", 1);
    for (body, accepted) in [(partial.as_str(), 24), ("<html>unrecognized page</html>", 0)] {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path()).expect("store");
        let team = milesplit::parse_team_index(WI_TEAMS_FIXTURE).expect("index").remove(0);
        seed_cache(&store.http_cache_dir(), &format!("{}/roster", team.url), body);
        let fetcher = Fetcher::new(store.http_cache_dir(), None, Duration::from_millis(1),
            std::collections::HashMap::new(), Vec::new()).expect("fetcher");
        let options = collect_options();
        let first = census::collect_state_rosters(&fetcher, &store, std::slice::from_ref(&team),
            &options, UsJurisdiction::Wisconsin).await.expect("persisting source outcome");
        assert_eq!(first.rosters_committed, 0);
        assert_eq!(first.rosters_remaining, 1);
        assert_eq!(first.athletes, accepted);
        assert_eq!(first.errors.len(), 1);
        let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes");
        assert_eq!(athletes.len(), accepted);
        let observations: Vec<SourceObservation> = store.scan(Table::SourceObservations).expect("observations");
        assert_eq!(observations.iter().filter(|row| matches!(row, SourceObservation::Athlete(_))).count(),
            accepted);
        let expected_observations = accepted + usize::from(accepted != 0);
        assert_eq!(observations.len(), expected_observations);
        drop(store);
        let reopened = Store::open(dir.path()).expect("reopening durable outcomes");
        let resumed = census::collect_state_rosters(&fetcher, &reopened, std::slice::from_ref(&team),
            &options, UsJurisdiction::Wisconsin).await.expect("resuming");
        assert_eq!(resumed.errors, first.errors);
        assert_eq!(resumed.rosters_committed, 0);
        assert_eq!(resumed.rosters_remaining, 1);
        assert_eq!(resumed.rosters_skipped, 1);
        assert_eq!(resumed.athletes, accepted);
        let persisted: Vec<SourceObservation> = reopened.scan(Table::SourceObservations).expect("observations");
        assert_eq!(persisted.len(), expected_observations);
        let stats = fetcher.stats().await;
        assert_eq!(stats.requests, 0);
        assert_eq!(stats.cache_hits, 1);
    }
}
