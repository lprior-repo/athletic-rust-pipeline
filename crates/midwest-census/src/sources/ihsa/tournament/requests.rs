//! Every request this adapter makes, and how a failure is recorded.
//!
//! Each helper answers `None` when the request or the payload did not land, after recording the
//! failure on the report as one error plus one note naming the document — so the walk keeps its shape
//! and no caller has to remember the two. The document is fetched through the shared
//! [`AdapterContext`] fetcher, which owns pacing, the on-disk body cache and the evidence record.

use super::parse;
use super::wire::{EventSummary, EventsEnvelope, MeetRow};
use crate::sources::ihsa::IHSA_API;
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};

/// One document's body, or `None` when the request did not land.
pub(super) async fn body(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
    what: &str,
) -> Option<String> {
    match ctx.fetcher.get(url, &ctx.fetch_options()).await {
        Ok(outcome) => Some(outcome.text()),
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("{what}: {url} failed: {error}"));
            None
        }
    }
}

/// Record a decode failure as an error and a note; `None` when the payload did not decode.
pub(super) fn decoded<T>(
    report: &mut AdapterReport,
    url: &str,
    what: &str,
    parsed: CrawlResult<T>,
) -> Option<T> {
    match parsed {
        Ok(value) => Some(value),
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("{what} at {url} did not decode: {error}"));
            None
        }
    }
}

/// The meets index.
pub(super) async fn meets_index(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
) -> Option<Vec<MeetRow>> {
    let url = format!("{IHSA_API}/v1/track-field/meets");
    let body = body(ctx, report, &url, "track-field meets index").await?;
    let parsed = parse::parse_meets(&body);
    decoded(report, &url, "the track-field meets index", parsed)
}

/// One meet's events index.
pub(super) async fn events(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<EventsEnvelope> {
    let body = body(ctx, report, url, "track-field events index").await?;
    let parsed = parse::parse_events(&body);
    decoded(report, url, "the track-field events index", parsed)
}

/// One event's summary.
pub(super) async fn summary(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<EventSummary> {
    let body = body(ctx, report, url, "event summary").await?;
    let parsed = parse::parse_summary(&body);
    decoded(report, url, "the event summary", parsed)
}

/// The body of one cross-country qualifier list; the archive's error envelope decodes as a body and
/// is the caller's to read as a note.
pub(super) async fn qualifiers(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<String> {
    body(ctx, report, url, "cross-country qualifiers").await
}

/// The newest completed term, the only one whose cross-country archive exists.
pub(super) async fn newest_term(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
) -> Option<String> {
    let url = format!("{IHSA_API}/v1/terms");
    let body = body(ctx, report, &url, "terms").await?;
    let parsed = parse::newest_term(&body);
    decoded(report, &url, "the terms list", parsed).flatten()
}

/// The events index of one meet.
pub(super) fn events_url(row: &MeetRow) -> String {
    format!(
        "{IHSA_API}/v1/track-field/meets/{}/events?gender={}",
        row.year, row.gender
    )
}

/// One event's summary.
pub(super) fn summary_url(event_id: &str) -> String {
    format!("{IHSA_API}/v1/track-field/events/{event_id}/summary")
}

/// One cross-country state-final list.
pub(super) fn qualifiers_url(term: &str, tournament_id: u32) -> String {
    format!("{IHSA_API}/v1/{term}/statefinal/cc-qualifiers?tournamentId={tournament_id}")
}
