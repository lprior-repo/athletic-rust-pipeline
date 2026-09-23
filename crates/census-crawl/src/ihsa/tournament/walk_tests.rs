//! The walk end to end, over the committed captures and a fresh store.
//!
//! Every request the walk makes is seeded into the fetcher's on-disk cache under the key
//! [`crate::net::Fetcher`] derives, so a run that leaves the machine reports a live request and fails
//! the assertion below instead of quietly passing on a socket. The captures are the real payloads in
//! `tests/fixtures/ihsa_tournament/`:
//!
//! | capture | serves |
//! |---|---|
//! | `track_field_meets.json` | `GET /v1/track-field/meets` (both 2026 state finals) |
//! | `track_field_2026_boys_events.json` | `GET /v1/track-field/meets/2026/events?gender=Boys`, trimmed to three index rows so the walk stays at two summaries |
//! | `event_2790204_boys_hj_1a_finals.json` | the 20-finisher individual summary |
//! | `event_500937_boys_4x800_1a_finals.json` | the 12-team relay summary (48 legs) |
//! | `terms.json` | `GET /v1/terms` (`2025-26` newest) |
//! | `cc_qualifiers_2025-26_{688,689,690,691}.json` | the four captured state-finalist lists |
//! | `cc_qualifiers_archive_error.json` | the archive's own error envelope, served for `692`/`693` |
//!
//! The two ids the fixture corpus has no capture for are served the archive's real error body, which
//! is what the walk asserts about them: a note, never an error. The trimmed events index keeps the
//! third row (`2790203`) with its `hasResults` flipped to `false`, so the "indexed without results"
//! path is walked too.
use super::collect;
use crate::ihsa::Options;
use crate::{AdapterContext, AdapterReport};
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, CanonicalSchool, Grade, Mark,
    SchoolYear, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

const FIXTURE_MEETS: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/track_field_meets.json");
const FIXTURE_EVENTS: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/track_field_2026_boys_events.json");
const FIXTURE_HJ: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/event_2790204_boys_hj_1a_finals.json");
const FIXTURE_RELAY: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/event_500937_boys_4x800_1a_finals.json");
const FIXTURE_TERMS: &str = include_str!("../../../tests/fixtures/ihsa_tournament/terms.json");
const FIXTURE_XC_688: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_688.json");
const FIXTURE_XC_689: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_689.json");
const FIXTURE_XC_690: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_690.json");
const FIXTURE_XC_691: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_691.json");
const FIXTURE_ARCHIVE_ERROR: &str =
    include_str!("../../../tests/fixtures/ihsa_tournament/cc_qualifiers_archive_error.json");

const OBSERVED_ON: &str = "2026-09-20";
const API: &str = "https://api.ihsa.org";

/// The events index trimmed to the three rows the walk below walks: the high-jump final (three index
/// rows of the same event are one event instance), the relay final, and one row whose `hasResults`
/// is flipped so the "indexed, no results" path is exercised without a fourth summary request.
fn trimmed_events() -> String {
    let mut document: serde_json::Value =
        serde_json::from_str(FIXTURE_EVENTS).expect("fixture is JSON");
    let rows = document
        .get_mut("data")
        .and_then(serde_json::Value::as_array_mut)
        .expect("the events index is an array");
    rows.retain(|row| {
        matches!(
            row.get("eventId").and_then(serde_json::Value::as_str),
            Some("2790204" | "500937" | "2790203")
        )
    });
    for row in rows.iter_mut() {
        if row.get("eventId").and_then(serde_json::Value::as_str) == Some("2790203") {
            row["hasResults"] = serde_json::Value::Bool(false);
        }
    }
    document["count"] = serde_json::Value::from(rows.len());
    serde_json::to_string(&document).expect("trimmed index serializes")
}

/// One seeded run's store, kept alive with its temp directory.
struct Harness {
    dir: tempfile::TempDir,
    store: Store,
}

impl Harness {
    /// A store whose fetcher cache holds every capture the walk asks for.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = dir.path().join("http");
        let list = |id: u32| format!("{API}/v1/2025-26/statefinal/cc-qualifiers?tournamentId={id}");
        seed(
            &cache,
            &[
                (format!("{API}/v1/track-field/meets"), FIXTURE_MEETS),
                (
                    format!("{API}/v1/track-field/meets/2026/events?gender=Boys"),
                    &trimmed_events(),
                ),
                (
                    format!("{API}/v1/track-field/events/2790204/summary"),
                    FIXTURE_HJ,
                ),
                (
                    format!("{API}/v1/track-field/events/500937/summary"),
                    FIXTURE_RELAY,
                ),
                (format!("{API}/v1/terms"), FIXTURE_TERMS),
                (list(688), FIXTURE_XC_688),
                (list(689), FIXTURE_XC_689),
                (list(690), FIXTURE_XC_690),
                (list(691), FIXTURE_XC_691),
                (list(692), FIXTURE_ARCHIVE_ERROR),
                (list(693), FIXTURE_ARCHIVE_ERROR),
            ],
        );
        let store = Store::open(dir.path().join("store")).expect("store");
        Self { dir, store }
    }

    /// One run of the adapter over the seeded cache.
    async fn run(&self, options: &Options) -> AdapterReport {
        let cache = self.dir.path().join("http");
        let fetcher = crate::net::Fetcher::new(
            &cache,
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
            Vec::new(),
        )
        .expect("fetcher");
        let ctx = AdapterContext {
            fetcher: &fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2025).expect("2025 is a season"),
            observed_on: OBSERVED_ON.to_string(),
        };
        collect(&ctx, options)
            .await
            .expect("the walk returns a report")
    }

    fn scan<T: census_store::Entity>(&self, table: Table) -> Vec<T> {
        self.store.scan(table).expect("scan")
    }
}

/// The options a full walk uses: the meets index publishes both 2026 state finals, but only the boys
/// meet has captured payloads, so the walk is limited to the first row it publishes. The cross-country
/// lists are not per-meet work and are read whatever the limit.
fn options() -> Options {
    Options {
        limit: Some(1),
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    }
}

/// Seed the fetcher's on-disk cache for `url` under the key the fetcher derives
/// (`sha256(method \x1f url \x1f body)[..16]`).
fn seed(cache: &std::path::Path, bodies: &[(String, &str)]) {
    use sha2::{Digest, Sha256};
    for (url, body) in bodies {
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
            "fetched_at": "2026-09-20T14:39:00Z",
        });
        std::fs::create_dir_all(cache).expect("cache dir");
        std::fs::write(cache.join(format!("{key}.body")), body).expect("cache body");
        std::fs::write(
            cache.join(format!("{key}.meta.json")),
            serde_json::to_vec(&meta).expect("cache meta"),
        )
        .expect("cache meta written");
    }
}

/// The one run: 11 requests (index, events, two summaries, terms, six lists), all cached, and the
/// entities the four captured lists and the two captured summaries yield.
#[tokio::test]
async fn walk_reads_the_captured_meet_and_the_six_lists() {
    let harness = Harness::new();
    let report = harness.run(&options()).await;

    assert_eq!(report.errors, 0, "notes: {:?}", report.notes);
    assert_eq!(
        report.requests, 0,
        "every request is answered from the seeded cache"
    );
    assert_eq!(
        report.from_cache, 11,
        "index, events, 2 summaries, terms, 6 lists"
    );
    assert_eq!(
        report.rows, 20,
        "the 20 individual finishers become performances"
    );
    assert_eq!(report.unit, "performances");

    let notes = report.notes.join("\n");
    for expected in [
        "track & field: 1 meets walked, 0 skipped at their published LastRefreshedAt, 3 events minted (1 without results), 2 event summaries read",
        "finishers: 32 rows, 0 without a mark, 0 without a grade, 0 without a resolvable school, 48 relay legs minted with no performance row of their own",
        "cross country: 1571 qualifier rows, 1569 of them with a published grade, 2 without (an athlete keys on its graduating class, so those mint nothing), 0 lists already read",
        "canonical: 240 schools (240 minted here), 1 meets, 3 events, 145 teams, 1636 athletes, 20 performances",
        "tournamentId 692 (2A, girls): Archive not available for term 2026-27",
    ] {
        assert!(notes.contains(expected), "missing {expected:?} in:\n{notes}");
    }

    assert_eq!(harness.scan::<CanonicalMeet>(Table::Meets).len(), 1);
    assert_eq!(
        harness
            .scan::<CanonicalPerformance>(Table::Performances)
            .len(),
        20
    );
    assert_eq!(harness.scan::<CanonicalSchool>(Table::Schools).len(), 240);
    let athletes = harness.scan::<CanonicalAthlete>(Table::Athletes);
    assert_eq!(
        athletes.len(),
        1610,
        "the run appends 1636 athlete rows, of which 26 are the same person on both surfaces \
         (a state-final finisher who also ran the cross-country state final for the same school and \
         class) and merge onto one canonical athlete: {} rows",
        athletes.len()
    );
    let dual = athletes
        .iter()
        .filter(|athlete| {
            let has = |wanted: SourceNamespace| {
                athlete
                    .source_identities
                    .iter()
                    .any(|identity| identity.namespace == wanted)
            };
            has(SourceNamespace::AthleticNet {
                kind: "athlete".to_string(),
            }) && has(SourceNamespace::AssociationAthlete {
                association: "ihsa".to_string(),
            })
        })
        .count();
    assert!(
        dual > 0,
        "an athlete who appears on both surfaces carries both ids: {dual} did"
    );
}

/// The published ids land as identity rows: Athletic.net's athlete/team ids from the summaries, the
/// school's own id from every row, and the archive's entry number from the cross-country lists.
#[tokio::test]
async fn mapped_ids_are_athleticnet_and_ihsa_identity_rows() {
    let harness = Harness::new();
    harness.run(&options()).await;

    let meets = harness.scan::<CanonicalMeet>(Table::Meets);
    let meet = meets.first().expect("one meet");
    assert_eq!(meet.name, "2026 IHSA Boys State Track & Field");
    assert_eq!(
        meet.date, "2026-05-28",
        "the earliest event date of the meet"
    );
    assert_eq!(meet.end_date.as_deref(), Some("2026-05-30"));
    assert!(
        meet.source_identities
            .iter()
            .any(|identity| identity.namespace
                == SourceNamespace::AthleticNet {
                    kind: "live".to_string()
                }
                && identity.id == "74003"
                && identity.url.as_deref() == Some("https://live.athletic.net/meets/74003")),
        "the meet carries its Athletic.net Live id and page: {:?}",
        meet.source_identities
    );

    let athletes = harness.scan::<CanonicalAthlete>(Table::Athletes);
    let kinds = |kind: &str| {
        athletes
            .iter()
            .filter(|athlete| {
                athlete.source_identities.iter().any(|identity| {
                    identity.namespace
                        == SourceNamespace::AthleticNet {
                            kind: kind.to_string(),
                        }
                })
            })
            .count()
    };
    assert_eq!(
        kinds("athlete"),
        68,
        "20 individual finishers + 48 relay legs"
    );
    assert_eq!(
        kinds("live"),
        68,
        "every captured finisher publishes both ids"
    );

    let winner = athletes
        .iter()
        .find(|athlete| {
            athlete.source_identities.iter().any(|identity| {
                identity.namespace
                    == SourceNamespace::AthleticNet {
                        kind: "athlete".to_string(),
                    }
                    && identity.id == "27740691"
            })
        })
        .expect("the captured high-jump winner");
    assert_eq!(winner.canonical_name, "Kehlin Crawford");
    assert!(
        winner
            .observed_grades
            .iter()
            .any(
                |observation| observation.grade == Grade::new(11).expect("grade 11")
                    && observation.school_year == SchoolYear::new(2025).expect("2025 is a season")
            ),
        "grade 11 observed in 2025-26: {:?}",
        winner.observed_grades
    );

    let performances = harness.scan::<CanonicalPerformance>(Table::Performances);
    let top = performances
        .iter()
        .find(|performance| performance.mark == Mark::DistanceMetres(2.02))
        .expect("the captured 2.02m high jump");
    assert_eq!(top.place, Some(1));
    assert_eq!(top.athlete, winner.id);
    assert_eq!(top.date, "2026-05-30");
    assert_eq!(top.observed_grade, Grade::new(11));
    assert!(
        performances
            .iter()
            .all(|performance| matches!(performance.mark, Mark::DistanceMetres(_))),
        "a relay's own mark is the team's, so no leg is charged with it"
    );

    let byron = athletes
        .iter()
        .find(|athlete| {
            athlete.source_identities.iter().any(|identity| {
                identity.namespace
                    == SourceNamespace::AssociationAthlete {
                        association: "ihsa".to_string(),
                    }
                    && identity.id == "688:471"
            })
        })
        .expect("the captured boys 1A entry 471");
    assert_eq!(byron.canonical_name, "Alex Booker");
    let school = harness
        .scan::<CanonicalSchool>(Table::Schools)
        .into_iter()
        .find(|school| school.id == byron.school)
        .expect("the qualifier's school");
    assert_eq!(school.name, "Byron");
    assert_eq!(school.association.as_deref(), Some("ihsa"));
    assert!(
        school.source_identities.iter().any(|identity| identity
            == &census_domain::model::SourceIdentity::new(
                SourceNamespace::AssociationSchool {
                    association: "ihsa".to_string()
                },
                "0247"
            )),
        "the school keeps the association's own id: {:?}",
        school.source_identities
    );
}

/// A second run over the same store: the meets index says the one meet is unchanged, the four lists
/// the first run read are journaled, and the two the archive answered with an error are asked again.
#[tokio::test]
async fn second_run_resumes_on_the_published_change_signal() {
    let harness = Harness::new();
    let first = harness.run(&options()).await;
    assert_eq!(first.rows, 20);

    let second = harness.run(&options()).await;
    assert_eq!(second.errors, 0, "notes: {:?}", second.notes);
    assert_eq!(second.requests, 0, "a resumed run reads nothing live");
    assert_eq!(
        second.from_cache, 4,
        "meets index, terms, and the two lists the archive errors on"
    );
    assert_eq!(second.rows, 0, "no summary is read again");
    let notes = second.notes.join("\n");
    assert!(notes.contains("0 meets walked, 1 skipped"), "{notes}");
    assert!(notes.contains("4 lists already read"), "{notes}");
    assert!(
        notes.contains("tournamentId 693 (3A, girls): Archive not available"),
        "{notes}"
    );
    assert_eq!(
        harness
            .scan::<CanonicalPerformance>(Table::Performances)
            .len(),
        20,
        "the resumed run appends no second copy"
    );
}

/// A run scoped to another state spends no request and says why.
#[tokio::test]
async fn out_of_scope_states_spend_no_request() {
    let harness = Harness::new();
    let scoped = Options {
        states: vec![UsJurisdiction::Minnesota],
        ..options()
    };
    let report = harness.run(&scoped).await;
    assert_eq!(report.requests, 0);
    assert_eq!(report.errors, 0);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("does not include IL")),
        "the report names the filter: {:?}",
        report.notes
    );
    assert_eq!(
        harness
            .scan::<CanonicalPerformance>(Table::Performances)
            .len(),
        0
    );
}
