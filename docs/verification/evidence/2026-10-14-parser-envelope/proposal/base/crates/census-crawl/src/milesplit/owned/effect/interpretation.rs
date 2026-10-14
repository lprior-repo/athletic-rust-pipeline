use super::{journal, OwnedMeetOutcome, OwnedMeetVerdict, OWNED_MEET_PHASE};
use crate::milesplit::owned::OwnedMeetPage;
use crate::{AdapterContext, CrawlError, CrawlResult};
use serde::Serialize;

pub(super) fn record(
    ctx: &AdapterContext<'_>,
    content_key: &str,
    manifest_key: &str,
    outcome: &OwnedMeetOutcome,
) -> CrawlResult<()> {
    let acquisition =
        manifest_key
            .strip_suffix("/manifest")
            .ok_or_else(|| CrawlError::Invariant {
                detail: "owned acquisition manifest key lacks suffix".into(),
            })?;
    let (disposition, summary) = summary(ctx, content_key, outcome)?;
    let receipt = format!("{disposition}/{acquisition}");
    let mut batch = ctx.write_batch();
    if journal::stage_capture(ctx, &mut batch, OWNED_MEET_PHASE, &receipt, &summary)? {
        batch.commit()?;
    }
    Ok(())
}

fn summary(
    ctx: &AdapterContext<'_>,
    key: &str,
    outcome: &OwnedMeetOutcome,
) -> CrawlResult<(&'static str, serde_json::Value)> {
    match &outcome.verdict {
        OwnedMeetVerdict::Parsed(page) => {
            record_rows(ctx, key, page)?;
            let disposition = if page.individual_parse_complete() {
                "parsed"
            } else {
                "partial"
            };
            Ok((disposition, parsed_summary(outcome, page)))
        }
        verdict => Ok((
            "partial",
            serde_json::json!({ "capture": &outcome.capture, "verdict": verdict }),
        )),
    }
}

fn record_rows(ctx: &AdapterContext<'_>, key: &str, page: &OwnedMeetPage) -> CrawlResult<()> {
    page.rows
        .iter()
        .try_for_each(|row| commit_source(ctx, &format!("row/{key}/{}", row.locator), row))?;
    page.rejected
        .iter()
        .try_for_each(|row| commit_source(ctx, &format!("rejected/{key}/{}", row.locator), row))
}

fn commit_source<T: Serialize>(ctx: &AdapterContext<'_>, key: &str, source: &T) -> CrawlResult<()> {
    super::super::parse::admission::recording(ctx, source)?;
    let mut batch = ctx.write_batch();
    if record_source(ctx, &mut batch, key, source)? {
        batch.commit()?;
    }
    Ok(())
}

fn record_source<T: Serialize>(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    legacy_key: &str,
    source: &T,
) -> CrawlResult<bool> {
    let payload = journal::payload(OWNED_MEET_PHASE, source)?;
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(OWNED_MEET_PHASE, legacy_key, |prior| {
            Ok(prior == Some(&payload))
        })? {
            return Ok(false);
        }
    }
    if ctx
        .store
        .journal_payload(OWNED_MEET_PHASE, legacy_key)?
        .is_some_and(|prior| prior == payload)
    {
        return Ok(false);
    }
    let key = format!("{legacy_key}/{}", journal::digest(&payload)?);
    journal::stage(ctx, batch, OWNED_MEET_PHASE, &key, &payload)
}

fn parsed_summary(outcome: &OwnedMeetOutcome, page: &OwnedMeetPage) -> serde_json::Value {
    let complete = page.individual_parse_complete();
    let team_relays = page
        .rejected
        .iter()
        .filter(|row| row.kind == crate::milesplit::owned::OwnedRejectionKind::TeamRelay)
        .count();
    serde_json::json!({
        "disposition": if complete { "parsed" } else { "partial" },
        "capture": &outcome.capture, "published_rows": page.published_rows,
        "owned_rows": page.rows.len(), "team_relay_rows": team_relays,
        "rejected_individual_rows": page.rejected.len().saturating_sub(team_relays),
        "individual_parse_complete": complete,
        "completeness": page.completeness, "ownership_complete": page.ownership_complete(),
    })
}
