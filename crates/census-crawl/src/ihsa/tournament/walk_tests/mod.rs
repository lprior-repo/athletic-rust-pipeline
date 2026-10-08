use super::collect;
use crate::ihsa::Options;
use crate::{AdapterContext, AdapterReport};
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, CanonicalSchool, SchoolYear,
    SourceNamespace,
};
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FIXTURE_MEETS: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/track_field_meets.json");
const FIXTURE_EVENTS: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/track_field_2026_boys_events.json");
const FIXTURE_HJ: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/event_2790204_boys_hj_1a_finals.json");
const FIXTURE_RELAY: &str = include_str!(
    "../../../../tests/fixtures/ihsa_tournament/event_500937_boys_4x800_1a_finals.json"
);
const FIXTURE_TERMS: &str = include_str!("../../../../tests/fixtures/ihsa_tournament/terms.json");
const FIXTURE_XC_688: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_688.json");
const FIXTURE_XC_689: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_689.json");
const FIXTURE_XC_690: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_690.json");
const FIXTURE_XC_691: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_691.json");
const FIXTURE_ARCHIVE_ERROR: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_archive_error.json");

const OBSERVED_ON: &str = "2026-09-20";
const API: &str = "https://api.ihsa.org";

fn trimmed_events() -> TestResult<String> {
    let mut document: serde_json::Value = serde_json::from_str(FIXTURE_EVENTS)?;
    let rows = document
        .get_mut("data")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("the events index is an array")?;
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
    Ok(serde_json::to_string(&document)?)
}

struct Harness {
    dir: tempfile::TempDir,
    store: Store,
}

impl Harness {
    fn new() -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let cache = dir.path().join("http");
        let list = |id: u32| format!("{API}/v1/2025-26/statefinal/cc-qualifiers?tournamentId={id}");
        seed(
            &cache,
            &[
                (format!("{API}/v1/track-field/meets"), FIXTURE_MEETS),
                (
                    format!("{API}/v1/track-field/meets/2026/events?gender=Boys"),
                    &trimmed_events()?,
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
        )?;
        let store = Store::open(dir.path().join("store"))?;
        Ok(Self { dir, store })
    }

    async fn run(&self, options: &Options) -> TestResult<AdapterReport> {
        let cache = self.dir.path().join("http");
        let fetcher = crate::net::Fetcher::new(
            &cache,
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
            Vec::new(),
        )?;
        let ctx = AdapterContext {
            fetcher: &fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2025).ok_or("2025 is a season")?,
            observed_on: OBSERVED_ON.to_string(),
            recording: None,
        };
        Ok(collect(&ctx, options).await?)
    }

    fn scan<T: census_store::Entity>(&self, table: Table) -> TestResult<Vec<T>> {
        Ok(self.store.scan(table)?)
    }
}

fn options() -> Options {
    Options {
        limit: Some(1),
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    }
}

fn seed(cache: &std::path::Path, bodies: &[(String, &str)]) -> TestResult {
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
            "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
            "bytes": body.len(),
            "fetched_at": "2026-09-20T14:39:00Z",
        });
        std::fs::create_dir_all(cache)?;
        std::fs::write(cache.join(format!("{key}.body")), body)?;
        std::fs::write(
            cache.join(format!("{key}.meta.json")),
            serde_json::to_vec(&meta)?,
        )?;
    }
    Ok(())
}

#[test]
fn walk_reads_the_captured_meet_and_the_six_lists() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new()?;
            let report = harness.run(&options()).await?;

            check!(eq; report.errors, 0, "notes: {:?}", report.notes);
            check!(eq;
                report.requests, 0,
                "every request is answered from the seeded cache"
            );
            check!(eq;
                report.from_cache, 11,
                "index, events, 2 summaries, terms, 6 lists"
            );
            check!(eq;
                report.rows, 20,
                "the 20 individual finishers become performances"
            );
            check!(eq; report.unit, "performances");

            check!(eq; harness.scan::<CanonicalMeet>(Table::Meets)?.len(), 1);
            check!(eq;
                harness
                    .scan::<CanonicalPerformance>(Table::Performances)?
                    .len(),
                20
            );
            check!(eq; harness.scan::<CanonicalSchool>(Table::Schools)?.len(), 240);
            let athletes = harness.scan::<CanonicalAthlete>(Table::Athletes)?;
            check!(eq;
                athletes.len(),
                1637,
                "source-owned athletes are not merged by candidate name"
            );
            check!(eq;
                athletes
                    .iter()
                    .filter(|athlete| {
                        athlete.source.as_ref().is_some_and(|source| {
                            source.namespace == SourceNamespace::athletic_net("athlete")
                        })
                    })
                    .count(),
                68
            );
            check!(eq;
                athletes
                    .iter()
                    .filter(|athlete| {
                        athlete.source.as_ref().is_some_and(|source| {
                            source.namespace
                                == SourceNamespace::AssociationAthlete {
                                    association: "ihsa".to_owned(),
                                }
                        })
                    })
                    .count(),
                1569
            );
            let dual = athletes
                .iter()
                .filter(|athlete| {
                    let has = |wanted: SourceNamespace| {
                        athlete
                            .identities()
                            .any(|identity| identity.namespace == wanted)
                    };
                    has(SourceNamespace::AthleticNet {
                        kind: "athlete".to_string(),
                    }) && has(SourceNamespace::AssociationAthlete {
                        association: "ihsa".to_string(),
                    })
                })
                .count();
            check!(eq;
                dual, 0,
                "an unreviewed name match cannot combine independent source owners"
            );
            Ok(())
        })
}

mod identity;
mod ownership;
