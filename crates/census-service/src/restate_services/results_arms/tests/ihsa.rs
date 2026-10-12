use super::*;
use crate::capture_cache;
use census_domain::model::{CanonicalMeet, CanonicalPerformance, Mark};
use census_store::Table;

const API: &str = "https://api.ihsa.org";
const MEETS: &[u8] = include_bytes!(
    "../../../../../census-crawl/tests/fixtures/ihsa_tournament/track_field_meets.json"
);
const EVENTS: &str = include_str!(
    "../../../../../census-crawl/tests/fixtures/ihsa_tournament/track_field_2026_boys_events.json"
);
const HIGH_JUMP: &[u8] = include_bytes!(
    "../../../../../census-crawl/tests/fixtures/ihsa_tournament/event_2790204_boys_hj_1a_finals.json"
);
const RELAY: &[u8] = include_bytes!(
    "../../../../../census-crawl/tests/fixtures/ihsa_tournament/event_500937_boys_4x800_1a_finals.json"
);

struct Harness {
    _root: tempfile::TempDir,
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
}

impl Harness {
    fn new(meets: &[u8]) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let store = Arc::new(Store::open(root.path().join("store"))?);
        let fetcher = Arc::new(
            Fetcher::new(
                store.http_cache_dir(),
                None,
                std::time::Duration::ZERO,
                std::collections::HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true),
        );
        capture_cache::seed(
            fetcher.cache_dir(),
            &format!("{API}/v1/track-field/meets"),
            meets,
            "2026-09-20T14:39:00Z",
            &[],
        )?;
        capture_cache::seed(
            fetcher.cache_dir(),
            &format!("{API}/v1/terms"),
            b"{\"currentTerm\":\"2026-27\",\"terms\":[]}",
            "2026-09-20T14:39:00Z",
            &[],
        )?;
        Ok(Self {
            _root: root,
            store,
            fetcher,
        })
    }

    fn seed_results(&self) -> TestResult {
        let mut events: serde_json::Value = serde_json::from_str(EVENTS)?;
        let rows = events
            .get_mut("data")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("event rows")?;
        rows.retain(|row| {
            matches!(
                row.get("eventId").and_then(serde_json::Value::as_str),
                Some("2790204" | "500937")
            )
        });
        let count = rows.len();
        events["count"] = serde_json::json!(count);
        for (path, body) in [
            (
                "track-field/meets/2026/events?gender=Boys",
                serde_json::to_vec(&events)?,
            ),
            ("track-field/events/2790204/summary", HIGH_JUMP.to_vec()),
            ("track-field/events/500937/summary", RELAY.to_vec()),
            (
                "track-field/meets/2026/events?gender=Girls",
                b"{broken".to_vec(),
            ),
        ] {
            capture_cache::seed(
                self.fetcher.cache_dir(),
                &format!("{API}/v1/{path}"),
                &body,
                "2026-09-20T14:39:00Z",
                &[],
            )?;
        }
        Ok(())
    }

    async fn run(&self) -> TestResult<ResultsStageOutcome> {
        let mut historical = scope(2026)?;
        historical.jurisdiction = UsJurisdiction::Illinois;
        let Json(outcome) = results_stage(
            Arc::clone(&self.store),
            Arc::clone(&self.fetcher),
            historical,
            SourceSelection::parse(&["ihsa".to_string()])
                .map_err(crate::restate_services::tests::sdk_error)?,
        )
        .await
        .map_err(crate::restate_services::tests::sdk_error)?;
        Ok(outcome)
    }
}

#[tokio::test]
async fn ihsa_durable_results_retain_fixture_rows_and_quarantine_only_the_bad_meet() -> TestResult {
    let mut index: serde_json::Value = serde_json::from_slice(MEETS)?;
    index
        .get_mut("data")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("captured meet rows")?
        .reverse();
    let harness = Harness::new(&serde_json::to_vec(&index)?)?;
    harness.seed_results()?;
    let outcome = harness.run().await?;
    check!(eq; outcome.required_sources, vec!["ihsa"]);
    let source = outcome.per_source.first().ok_or("IHSA result arm")?;
    check!(eq; source.slug, "ihsa");
    check!(eq; source.rows, Some(20));
    check!(eq; source.errors, 1);
    check!(source
        .unfinished
        .iter()
        .any(|url| url.ends_with("events?gender=Girls")));
    check!(!outcome.is_terminal());
    let rows: Vec<CanonicalPerformance> = harness.store.scan(Table::Performances)?;
    check!(eq; rows.len(), 20);
    check!(rows
        .iter()
        .all(|row| matches!(row.mark, Mark::DistanceMetres(_))));
    check!(rows
        .iter()
        .all(|row| row.round.as_deref() == Some("Finals")));
    check!(eq; harness.store.walk_table(Table::RelayResults)?.rows, 12);
    let before = harness.store.walk_table(Table::Performances)?;
    let replay = harness.run().await?;
    check!(eq; replay.per_source.first().ok_or("replayed arm")?.rows, Some(0));
    check!(eq; harness.store.walk_table(Table::Performances)?, before);
    check!(eq; harness.fetcher.stats().await.physical_requests(), 0);
    check!(crate::restate_services::teams_arms::arm_for("ihsa").is_some());
    let Harness {
        _root,
        store,
        fetcher,
    } = harness;
    drop(fetcher);
    drop(store);
    let reopened = Store::open(_root.path().join("store"))?;
    let saved: Vec<CanonicalPerformance> = reopened.scan(Table::Performances)?;
    check!(eq; saved.len(), 20);
    check!(eq; saved, rows);
    check!(eq; reopened.walk_table(Table::Performances)?, before);
    let meets: Vec<CanonicalMeet> = reopened.scan(Table::Meets)?;
    check!(eq; meets.len(), 1);
    check!(meets
        .iter()
        .all(|meet| meet.state == Some(UsJurisdiction::Illinois)));
    check!(saved
        .iter()
        .all(|row| meets.iter().any(|meet| meet.id == row.meet)));
    Ok(())
}

#[tokio::test]
async fn ihsa_durable_empty_tournament_index_completes_without_fabricated_rows() -> TestResult {
    let harness = Harness::new(b"{\"count\":0,\"data\":[]}")?;
    let outcome = harness.run().await?;
    check!(eq; outcome.required_sources, vec!["ihsa"]);
    let source = outcome.per_source.first().ok_or("IHSA result arm")?;
    check!(eq; source.rows, Some(0));
    check!(eq; source.errors, 0);
    check!(outcome.is_terminal());
    check!(eq; harness.store.walk_table(Table::Performances)?.rows, 0);
    check!(eq; harness.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
