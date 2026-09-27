use super::parse;
use super::wire::{EventSummary, EventsEnvelope, MeetRow};
use crate::ihsa::IHSA_API;
use crate::{AdapterContext, AdapterReport, CrawlResult};

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

pub(super) async fn meets_index(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
) -> Option<Vec<MeetRow>> {
    let url = format!("{IHSA_API}/v1/track-field/meets");
    let body = body(ctx, report, &url, "track-field meets index").await?;
    let parsed = parse::parse_meets(&body);
    decoded(report, &url, "the track-field meets index", parsed)
}

pub(super) async fn events(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<EventsEnvelope> {
    let body = body(ctx, report, url, "track-field events index").await?;
    let parsed = parse::parse_events(&body);
    decoded(report, url, "the track-field events index", parsed)
}

pub(super) async fn summary(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<EventSummary> {
    let body = body(ctx, report, url, "event summary").await?;
    let parsed = parse::parse_summary(&body);
    decoded(report, url, "the event summary", parsed)
}

pub(super) async fn qualifiers(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<String> {
    body(ctx, report, url, "cross-country qualifiers").await
}

pub(super) async fn newest_term(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
) -> Option<String> {
    let url = format!("{IHSA_API}/v1/terms");
    let body = body(ctx, report, &url, "terms").await?;
    let parsed = parse::newest_term(&body);
    decoded(report, &url, "the terms list", parsed).flatten()
}

pub(super) fn events_url(row: &MeetRow) -> String {
    format!(
        "{IHSA_API}/v1/track-field/meets/{}/events?gender={}",
        row.year, row.gender
    )
}

pub(super) fn summary_url(event_id: &str) -> String {
    format!("{IHSA_API}/v1/track-field/events/{event_id}/summary")
}

pub(super) fn qualifiers_url(term: &str, tournament_id: u32) -> String {
    format!("{IHSA_API}/v1/{term}/statefinal/cc-qualifiers?tournamentId={tournament_id}")
}
