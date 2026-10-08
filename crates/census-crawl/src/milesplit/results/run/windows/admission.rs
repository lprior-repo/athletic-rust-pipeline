use super::super::projection::Input;
use super::{budget, Footprint, Stats};
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};

pub(super) fn context(input: &Input<'_>, metadata: &FetchOutcome) -> CrawlResult<Footprint> {
    Footprint::of(&(
        &input.reference.url,
        &input.page.meet.name,
        &input.page.meet.date,
        &input.page.meet.end_date,
        &input.page.skipped,
        &input.page.grade_issues,
        &input.page.region,
        &metadata.url,
        &metadata.response_url,
        &metadata.fetched_at,
        &metadata.content_type,
        &input.acquired.outcome.capture.url,
        &input.acquired.outcome.capture.response_url,
        &input.acquired.outcome.capture.fetched_at,
        &input.performance_as_of,
    ))
}

pub(super) fn row(input: &Input<'_>, index: usize) -> CrawlResult<Footprint> {
    let owned = input
        .acquired
        .result_set(&input.reference.rsid, input.performance_as_of)
        .ok_or_else(missing)?;
    Footprint::of(owned.page.rows.get(index).ok_or_else(missing)?)
}

pub(super) fn metadata(stats: &mut Stats, rows: usize) -> CrawlResult<()> {
    let requested = stats
        .unresolved
        .len()
        .checked_add(rows)
        .ok_or_else(missing)?;
    if requested > budget::METADATA_LABELS {
        return Err(CrawlError::Resource {
            resource: "result unresolved labels",
            requested,
            limit: budget::METADATA_LABELS,
        });
    }
    stats
        .unresolved
        .try_reserve(rows)
        .map_err(budget::reserve)?;
    stats
        .school_resolved
        .try_reserve(1)
        .map_err(budget::reserve)
}

pub(super) fn recording(ctx: &crate::AdapterContext<'_>, footprint: Footprint) -> CrawlResult<()> {
    if let Some(recording) = ctx.recording {
        let bytes = footprint.bytes.checked_add(16 * 1024).ok_or_else(missing)?;
        let work = budget::WINDOW_ROWS.checked_mul(16).ok_or_else(missing)?;
        recording.admit(bytes, work)?;
    }
    Ok(())
}

fn missing() -> CrawlError {
    CrawlError::Invariant {
        detail: "result admission locator missing or counter overflowed".into(),
    }
}
