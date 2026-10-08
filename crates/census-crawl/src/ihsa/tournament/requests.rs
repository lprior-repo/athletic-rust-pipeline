use super::parse;
use super::wire::{EventSummary, EventsEnvelope, MeetRow};
use crate::ihsa::IHSA_API;
use crate::{AdapterContext, AdapterReport, CrawlResult};

pub(super) async fn captured(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
    what: &str,
) -> Option<crate::net::FetchOutcome> {
    match ctx.fetcher.get(url, &ctx.fetch_options()).await {
        Ok(outcome) => Some(outcome),
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.unfinished.push(url.to_string());
            report.note(format!("{what}: {url} failed: {error}"));
            None
        }
    }
}
pub(super) async fn body(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
    what: &str,
) -> Option<String> {
    captured(ctx, report, url, what)
        .await
        .map(|outcome| outcome.text())
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
            report.unfinished.push(url.to_string());
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
) -> Option<(EventsEnvelope, crate::net::FetchOutcome)> {
    let capture = captured(ctx, report, url, "track-field events index").await?;
    let parsed = parse::parse_events(&capture.text());
    decoded(report, url, "the track-field events index", parsed).map(|events| (events, capture))
}

pub(super) async fn summary(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<(EventSummary, crate::net::FetchOutcome)> {
    let capture = captured(ctx, report, url, "event summary").await?;
    let parsed = capture.json().map_err(crate::CrawlError::from);
    decoded(report, url, "the event summary", parsed).map(|summary| (summary, capture))
}

pub(super) async fn qualifiers(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    url: &str,
) -> Option<crate::net::FetchOutcome> {
    captured(ctx, report, url, "cross-country qualifiers").await
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
