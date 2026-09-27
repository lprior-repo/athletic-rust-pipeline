use chrono::Datelike;
use restate_sdk::prelude::*;
use serde_json::Value;

use census_crawl::RecordedBatch;
use census_domain::UsJurisdiction;
use census_store::Table;

use super::ingest::IngestClient;
use super::wire::ingest::{IngestRequest, WindowRequest};
use super::MAX_ROWS_PER_REQUEST;

pub(super) fn endpoint_of(slug: &str, jurisdiction: UsJurisdiction) -> String {
    format!("{slug}_{}", jurisdiction.code().to_ascii_lowercase())
}

pub(super) fn window_of(at: &str) -> Result<String, HandlerError> {
    let day = chrono::NaiveDate::parse_from_str(at, "%Y-%m-%d").map_err(|source| {
        TerminalError::new(format!("run date {at} is not a calendar day: {source}"))
    })?;
    let week = day.iso_week();
    Ok(format!("{}-W{:02}", week.year(), week.week()))
}

pub(super) async fn post(
    ctx: &ObjectContext<'_>,
    endpoint: &str,
    window: &str,
    batches: &[RecordedBatch],
) -> Result<u64, HandlerError> {
    let object = ctx.object_client::<IngestClient>(endpoint);
    let run = ctx.invocation_id();
    let mut appended = 0_u64;
    for (batch_index, batch) in batches.iter().enumerate() {
        let table = batch.table;
        for (chunk_index, chunk) in units_of(batch).enumerate() {
            let Json(reply) = object
                .record(Json(IngestRequest {
                    table: table.file().to_string(),
                    rows: chunk.to_vec(),
                    operation_id: operation_of(
                        endpoint,
                        run,
                        window,
                        table,
                        batch_index,
                        chunk_index,
                    ),
                    cursor: None,
                }))
                .call()
                .await?;
            appended = appended.saturating_add(reply.appended);
        }
    }
    object
        .complete_window(Json(WindowRequest {
            window: window.to_string(),
        }))
        .call()
        .await?;
    Ok(appended)
}

fn operation_of(
    endpoint: &str,
    run: &str,
    window: &str,
    table: Table,
    batch_index: usize,
    chunk_index: usize,
) -> String {
    format!(
        "{endpoint}:{run}:{window}:{}:{batch_index}:{chunk_index}",
        table.file()
    )
}

fn units_of(batch: &RecordedBatch) -> impl Iterator<Item = &[Value]> {
    let chunks = batch.rows.chunks(MAX_ROWS_PER_REQUEST);
    let empty = batch.rows.get(..0).filter(|_| batch.rows.is_empty());
    chunks.chain(empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_endpoint_is_the_slugs_own_name_with_the_state() {
        assert_eq!(
            endpoint_of("wiaa_results", UsJurisdiction::Wisconsin),
            "wiaa_results_wi"
        );
        assert_eq!(endpoint_of("wayzata", UsJurisdiction::Iowa), "wayzata_ia");
    }

    #[test]
    fn a_window_is_the_iso_week_of_the_run_day() {
        assert_eq!(window_of("2026-09-23").expect("a calendar day"), "2026-W39");
        assert_eq!(window_of("2027-01-01").expect("a calendar day"), "2026-W53");
        assert_eq!(window_of("2026-01-01").expect("a calendar day"), "2026-W01");
    }

    #[test]
    fn a_run_day_that_is_not_a_calendar_day_refuses() {
        assert!(window_of("2026-09-31").is_err());
        assert!(window_of("yesterday").is_err());
    }
}
