//! Posting a walk's recorded rows through the deployment's own `Ingest` objects.
//!
//! This is the acquisition route the `Ingest` vocabulary describes: one durable object per
//! `<source>_<state>` endpoint, the rows a walk produced appended through `record`, and the week
//! they were read in declared complete through `complete_window`. A walk that runs this way writes
//! nothing itself — the object appends exactly the rows it was posted, into the same store — so a
//! routed acquisition is the same acquisition with a durable, readable counter on it, which is what
//! makes an offline run's claim that a source was acquired a *measured* claim instead of an inferred
//! one.
//!
//! Two ordering rules hold the route together, and both are the store's own rules:
//!
//! * A unit's journal entry is written only after the rows it produced are posted
//!   ([`super::jobs::flush_journal`]), so no unit is marked read whose rows are missing.
//! * The window is declared complete only after every batch landed, so a window is never closed over
//!   rows that were not appended.
//!
//! What the route deliberately does not do is make the two writers atomic: a crash between the
//! append and the marker re-reads the unit from cache and posts it again. The second post costs a
//! round trip and appends nothing — every post carries an operation id derived from the invocation,
//! the window and the unit's position, so the store recognizes the rows it already holds — and the
//! marker is then written once. Marking first instead would lose the rows, which is the failure the
//! census exists to avoid.

use chrono::Datelike;
use restate_sdk::prelude::*;
use serde_json::Value;

use census_crawl::RecordedBatch;
use census_domain::UsJurisdiction;
use census_store::Table;

use super::ingest::IngestClient;
use super::wire::ingest::{IngestRequest, WindowRequest};
use super::MAX_ROWS_PER_REQUEST;

/// The endpoint that serves one planned source's acquisition in one jurisdiction.
///
/// The slug is the plan's own spelling (`wiaa_results`, `wayzata`, …) and the state is the
/// jurisdiction's postal code, so `wiaa_results_wi` is a name an operator can derive from the plan
/// and read back with the same tools that read every other source object.
pub(super) fn endpoint_of(slug: &str, jurisdiction: UsJurisdiction) -> String {
    format!("{slug}_{}", jurisdiction.code().to_ascii_lowercase())
}

/// The window a day belongs to: its ISO week, spelled the way the sweep's vocabulary spells it
/// (`2026-W39`).
///
/// A day the calendar cannot read is a fault in the deployment's own clock rather than a source
/// condition, so it fails closed: a window label that named no week would close an endpoint's window
/// over a period nobody could read back.
pub(super) fn window_of(at: &str) -> Result<String, HandlerError> {
    let day = chrono::NaiveDate::parse_from_str(at, "%Y-%m-%d").map_err(|source| {
        TerminalError::new(format!("run date {at} is not a calendar day: {source}"))
    })?;
    let week = day.iso_week();
    Ok(format!("{}-W{:02}", week.year(), week.week()))
}

/// Post one source's recorded rows and close the window they were read in.
///
/// Batches are chunked to the per-request ceiling the object enforces, and the reply's count is
/// summed rather than assumed: what this returns is what the endpoint says it appended.
pub(super) async fn post(
    ctx: &ObjectContext<'_>,
    endpoint: &str,
    window: &str,
    batches: &[RecordedBatch],
) -> Result<u64, HandlerError> {
    let object = ctx.object_client::<IngestClient>(endpoint);
    // The name a re-post of one unit repeats, and a later walk does not: this invocation, the
    // endpoint and window it serves, the table, and the unit's position in the page the walk
    // produced. A crash between the append and the marker re-reads the unit from cache and posts it
    // again under this same name, which is exactly what the store's receipt answers.
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

/// The operation id one posted chunk carries.
///
/// Every part is fixed for the walk that produced it — the invocation, the endpoint, the window, the
/// table — or is the unit's own position in the page. A re-post of the same page therefore
/// reproduces the id exactly, and a later walk (a new invocation) produces different ones.
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

/// The units one batch's page becomes: chunks of the per-request ceiling, and a single empty unit
/// when the page held nothing.
///
/// The empty unit is not an optimisation's edge case: a source that answered a page with no rows has
/// still answered it, and the receipt its post leaves is what tells the next reader that the page was
/// posted rather than never reached. Without it, "no rows appended" and "this page was never
/// acquired" read the same.
fn units_of(batch: &RecordedBatch) -> impl Iterator<Item = &[Value]> {
    let chunks = batch.rows.chunks(MAX_ROWS_PER_REQUEST);
    let empty = batch.rows.get(..0).filter(|_| batch.rows.is_empty());
    chunks.chain(empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The endpoint name an operator derives from the plan is the name the run posts to.
    #[test]
    fn an_endpoint_is_the_slugs_own_name_with_the_state() {
        assert_eq!(
            endpoint_of("wiaa_results", UsJurisdiction::Wisconsin),
            "wiaa_results_wi"
        );
        assert_eq!(endpoint_of("wayzata", UsJurisdiction::Iowa), "wayzata_ia");
    }

    /// Windows are ISO weeks, including the year boundary, where the week and the calendar disagree
    /// about which year they belong to.
    #[test]
    fn a_window_is_the_iso_week_of_the_run_day() {
        assert_eq!(window_of("2026-09-23").expect("a calendar day"), "2026-W39");
        assert_eq!(window_of("2027-01-01").expect("a calendar day"), "2026-W53");
        assert_eq!(window_of("2026-01-01").expect("a calendar day"), "2026-W01");
    }

    /// A date the calendar cannot read refuses rather than closing a window nothing can look up.
    #[test]
    fn a_run_day_that_is_not_a_calendar_day_refuses() {
        assert!(window_of("2026-09-31").is_err());
        assert!(window_of("yesterday").is_err());
    }
}
