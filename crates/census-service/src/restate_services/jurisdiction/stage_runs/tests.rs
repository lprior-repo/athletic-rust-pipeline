use census_crawl::milesplit::{Season, Site};
use census_crawl::net::Fetcher;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use restate_sdk::prelude::Json;
use std::sync::Arc;

use crate::restate_services::history_stage::HistoricalStageScope;
use crate::restate_services::tests::sdk_error;
use crate::restate_services::wire::HistoryWindow;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn meet_source_durable_result_does_not_materialize_the_season() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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
            let site = Site::for_jurisdiction(UsJurisdiction::Ohio);
            for season in Season::ALL {
                for page in 1..=100u32 {
                    let body = format!(
                        r#"<meta name="application-name" content="MileSplit"><section class="meet-month" data-month="2026-09"><li class="meet-row" data-meet-id="{page}" data-filter-text="invite"><span class="meet-row__day">Sep 19</span><a class="meet-row__name" href="https://oh.milesplit.com/meets/{page}-invite/results">Invite {page}</a></li></section>{}"#,
                        if page < 100 { r#"<a rel="next">Next</a>"# } else { "" },
                    );
                    crate::capture_cache::seed(
                        fetcher.cache_dir(),
                        &site.results_url(season, 2026, page),
                        body.as_bytes(),
                        "2026-09-22T12:00:00Z",
                        &[],
                    )?;
                }
            }
            let scope = HistoricalStageScope {
                jurisdiction: UsJurisdiction::Ohio,
                year: 2026,
                refresh: false,
                window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
                observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
            };
            let baseline = Store::open(root.path().join("baseline"))?;
            let legacy = census_crawl::Recording::new();
            let fetch_options = census_crawl::net::FetchOptions::default();
            let at = scope.observed_on.to_string();
            let request = crate::census::MeetWalkRequest::new(scope.jurisdiction, scope.year, &at, &fetch_options);
            crate::census::collect_state_meets(&fetcher, &baseline, &request, Some(&legacy)).await?;
            let legacy = legacy.drain();
            let legacy_encoded_bytes = serde_json::to_vec(&legacy)?.len();
            let expected = source_rows(&legacy)?;
            census_crawl::recording::apply_recorded(&legacy, &baseline, "baseline")?;
            let mut emitted = Vec::new();
            let mut max_durable_bytes = 0usize;
            let mut census = crate::census::MeetCensus::default();
            for ordinal in 0..301 {
                let Json(outcome) = crate::restate_services::meets_arms::collect_fragment(
                    Arc::clone(&store), Arc::clone(&fetcher), scope,
                    ("milesplit".to_string(), ordinal),
                    format!("season-test/{ordinal}"),
                ).await.map_err(sdk_error)?;
                let encoded = serde_json::to_vec(&outcome)?;
                check!(encoded.len() <= 4096, "durable result retained {} bytes", encoded.len());
                max_durable_bytes = max_durable_bytes.max(encoded.len());
                if ordinal == 0 {
                    check!(eq; outcome.census.pages, 1);
                    check!(eq; outcome.census.frontiers.len(), 3);
                    check!(eq; outcome.census.frontiers.iter().map(|frontier| frontier.next_page).collect::<Vec<_>>(), vec![2, 1, 1]);
                    let before = fetcher.stats().await.cache_hits;
                    let Json(replayed) = crate::restate_services::meets_arms::collect_fragment(
                        Arc::clone(&store), Arc::clone(&fetcher), scope,
                        ("milesplit".to_string(), ordinal), format!("season-test/{ordinal}"),
                    ).await.map_err(sdk_error)?;
                    check!(eq; serde_json::to_vec(&replayed)?, encoded);
                    check!(eq; fetcher.stats().await.cache_hits, before);
                    check!(eq; store.walk_table(Table::SourceMeets)?.rows, 0);
                }
                census_crawl::recording::spool::verify_reference(&store, &outcome.identity, &outcome.spool)?;
                for index in 0..outcome.spool.fragments {
                    let key = census_crawl::recording::spool::fragment_key(&outcome.identity, index);
                    let recorded = census_crawl::recording::spool::read_fragment(&store, &outcome.identity, index)?;
                    check!(recorded.rows.iter().map(|batch| batch.rows.len()).sum::<usize>() <= 4096);
                    check!(serde_json::to_vec(&recorded)?.len() <= 8 * 1024 * 1024);
                    emitted.extend(source_rows(&recorded)?);
                    census_crawl::recording::apply_recorded(&recorded, &store, &key)?;
                    census_crawl::recording::apply_recorded(&recorded, &store, &key)?;
                }
                if ordinal == 0 {
                    check!(eq; store.walk_table(Table::SourceMeets)?.rows, 1);
                    check!(outcome.next.is_some(), "first ingest precedes source exhaustion");
                }
                census = crate::restate_services::meets_arms::absorb_fragment("milesplit", census, outcome.census).map_err(sdk_error)?;
                if outcome.next.is_none() {
                    break;
                }
            }
            check!(eq; census.rows, 300);
            check!(eq; store.walk_table(Table::SourceMeets)?.rows, 300);
            check!(census.is_terminal());
            check!(eq; emitted, expected);
            check!(eq; store.scan::<census_domain::model::SourceMeetRef>(Table::SourceMeets)?, baseline.scan::<census_domain::model::SourceMeetRef>(Table::SourceMeets)?);
            eprintln!("Meet durable probe: recorded fixture baseline={legacy_encoded_bytes} bytes; bounded durable maximum={max_durable_bytes}; first ingest=1 row; ordered rows={}", emitted.len());
            Ok(())
        })
}

fn source_rows(
    recorded: &census_store::recorded::Recorded,
) -> Result<Vec<census_domain::model::SourceMeetRef>, serde_json::Error> {
    recorded
        .rows
        .iter()
        .filter(|batch| batch.table == Table::SourceMeets)
        .flat_map(|batch| &batch.rows)
        .map(|row| serde_json::from_value(row.clone()))
        .collect()
}

#[test]
fn oversized_archived_row_is_refused_before_meet_ingest_apply() -> TestResult {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let recorded = census_store::recorded::Recorded {
        rows: vec![census_store::recorded::RecordedBatch {
            table: Table::SourceMeets,
            rows: vec![serde_json::json!({"id": "oversized", "name": "x".repeat(1024 * 1024)})],
        }],
        ..census_store::recorded::Recorded::default()
    };
    let encoded = serde_json::to_vec(&recorded)?;
    let key = census_crawl::recording::spool::fragment_key("oversized-send", 0);
    store.journal_done(
        census_crawl::recording::spool::FRAGMENT_PHASE,
        &key,
        &recorded,
    )?;
    store.journal_done(
        "meet_recorded_fragment_manifests_v1",
        &key,
        &serde_json::json!({
            "digest": format!("{:x}", Sha256::digest(&encoded)),
            "encoded_bytes": encoded.len(),
            "rows": 1,
        }),
    )?;
    let archived = census_crawl::recording::spool::read_fragment(&store, "oversized-send", 0)?;
    check!(eq; archived, recorded);
    let admitted = crate::restate_services::ingest_post::admit_recorded(&archived);
    check!(matches!(
        admitted,
        Err(census_store::StoreError::RowTooLarge { .. })
    ));
    check!(eq; store.walk_table(Table::SourceMeets)?.rows, 0);
    Ok(())
}

#[test]
fn selected_unarmed_meet_source_is_not_discarded() -> TestResult {
    let sources = crate::restate_services::source_selection::SourceSelection::parse(&[
        "milesplit".into(),
        "athleticlive".into(),
        "coach_directories".into(),
    ])
    .map_err(sdk_error)?;
    let slugs = super::selected_meet_sources(sources)
        .map(|source| source.slug)
        .collect::<Vec<_>>();
    check!(eq; slugs, vec!["athleticlive", "milesplit"]);
    Ok(())
}

#[test]
fn meet_fragment_identities_fit_the_ingest_operation_budget() -> TestResult {
    let digest =
        census_domain::model::serialized_digest(&("jurisdiction:VT:2026-27:1", "milesplit"))?;
    let base = super::meet_fragment_base(&digest).map_err(sdk_error)?;
    check!(eq; base.len(), super::MEET_IDENTITY_DIGEST_BYTES);
    let identity = super::meet_fragment_identity(base, 12).map_err(sdk_error)?;
    let recorded = census_store::recorded::Recorded {
        rows: vec![census_store::recorded::RecordedBatch {
            table: Table::SourceMeets,
            rows: vec![serde_json::json!({"id": "identity-budget"})],
        }],
        ..census_store::recorded::Recorded::default()
    };
    let prepared = census_crawl::recording::prepare_recorded(
        recorded,
        &census_crawl::recording::spool::fragment_key(&identity, 0),
    )?;
    check!(
        prepared.operation().len() <= crate::restate_services::wire::ingest::MAX_OPERATION_ID_BYTES
    );
    check!(super::meet_fragment_identity(&"a".repeat(64), 0).is_err());
    Ok(())
}
