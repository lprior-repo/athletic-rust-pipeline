//! What a roster pass leaves behind: one source observation per athlete MileSplit published, keyed by
//! MileSplit's own athlete id, carrying the school the source placed the athlete at and the name it
//! spelled.
//!
//! The walk is driven through the crate's own entry point — `census::collect_state_rosters`, the same
//! call the jurisdiction workflow's roster stage makes — over the committed WI captures seeded into
//! the fetcher's cache, so no socket is opened. The index fixture and the roster fixture are the
//! captured pair: `wi_teams_index.html` lists team `52649` first and `wi_roster_52649.html` is that
//! team's page. The walk is handed exactly that one team, so which school the observations name is
//! decided by the fixture rather than by the order a concurrent walk happens to finish in.
//!
//! Every expectation below comes out of the capture through MileSplit's own parser (`parse_roster`),
//! and every assertion reads the store's `SourceObservations` rows: delete the `observe_athletes_of`
//! call in `census/sweep/roster.rs` and this test fails on the first count.

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

/// Seed the fetcher's on-disk cache for `url`, so the walk runs with no socket: the key is
/// `sha256(method \x1f url \x1f body)[..16]`, the form `net::cache` writes.
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
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "application/json",
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
    let options = CollectOptions {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        limit_per_state: None,
        concurrency: 1,
        state_concurrency: 1,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
    };
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
        progress.rosters_done, 1,
        "the pass read the one team it was given"
    );

    // What the roster page itself publishes, through the parser the pass used: the provider's own
    // athlete ids, the names beside them, and the profile page each id was read from.
    let roster = milesplit::parse_roster(WI_ROSTER_FIXTURE, first.clone())
        .expect("the roster fixture parses");
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

    // The school the pass placed those athletes at, which is the school row it appended beside them.
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

    // The canonical rows the same pass minted, so each observation names a provider object the store
    // can be asked about: the athlete rows carry MileSplit's own id for this source.
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes read");
    assert_eq!(athletes.len(), published.len());
    for athlete in &athletes {
        let identity = athlete
            .source_identities
            .iter()
            .find(|identity| identity.namespace == SourceNamespace::MilesplitAthlete)
            .expect("the athlete row carries the provider's own id");
        assert!(
            filed.contains_key(identity.id.as_str()),
            "the observation and the canonical row name the same provider object: {}",
            identity.id
        );
    }

    // The first athlete the page lists, spelled out: the id in the profile link, the school that
    // owns the roster, and the name the page printed beside them.
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
