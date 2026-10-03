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
    let mut batch = ctx.write_batch();
    let (rows_changed, disposition, summary) = match &outcome.verdict {
        OwnedMeetVerdict::Parsed(page) => {
            let changed = record_rows(ctx, &mut batch, content_key, page)?;
            let complete = page.individual_parse_complete();
            let disposition = if complete { "parsed" } else { "partial" };
            (changed, disposition, parsed_summary(outcome, page))
        }
        verdict => (
            false,
            "partial",
            serde_json::json!({
                "capture": &outcome.capture, "verdict": verdict,
            }),
        ),
    };
    let receipt = format!("{disposition}/{acquisition}");
    let summary_changed =
        journal::stage_capture(ctx, &mut batch, OWNED_MEET_PHASE, &receipt, &summary)?;
    if rows_changed || summary_changed {
        batch.commit()?;
    }
    Ok(())
}

fn record_rows(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    key: &str,
    page: &OwnedMeetPage,
) -> CrawlResult<bool> {
    let rows = page.rows.iter().try_fold(false, |changed, row| {
        let staged = record_source(ctx, batch, &format!("row/{key}/{}", row.locator), row)?;
        Ok::<_, CrawlError>(changed || staged)
    })?;
    let rejected = page.rejected.iter().try_fold(false, |changed, rejection| {
        let staged = record_source(
            ctx,
            batch,
            &format!("rejected/{key}/{}", rejection.locator),
            rejection,
        )?;
        Ok::<_, CrawlError>(changed || staged)
    })?;
    Ok(rows || rejected)
}

fn record_source<T: Serialize>(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    legacy_key: &str,
    source: &T,
) -> CrawlResult<bool> {
    let payload = journal::payload(OWNED_MEET_PHASE, source)?;
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
