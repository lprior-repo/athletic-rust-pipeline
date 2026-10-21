use super::{Result, RetainedHtmlCaptureV1, RetainedResultOwnershipError};
use crate::milesplit::SourceRowLocator;
use census_domain::model::{serialized_digest, Gender, Grade, Sport};
use std::ops::Range;

mod cells;
mod context;
mod marks;
mod native_links;
mod row;
mod scan;
mod types;
mod visibility;

pub use types::{CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1};

const INTERPRETATION: &str = "retained-tfrrs-list-row/v1";

pub fn read_retained_tfrrs_list_rows<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    requested: &[SourceRowLocator],
) -> Result<CheckedRetainedTfrrsListV1<'a>> {
    super::selection::admit_count(requested.len())?;
    let body = capture.body()?;
    super::selection::validate_spans(&body, requested, &(0..body.len()))?;
    let (base, list_id, year, sport) = native_links::list(capture.requested_url())?;
    let selected = scan::selected(&body, requested)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(selected.len())
        .map_err(|_| RetainedResultOwnershipError::Allocation)?;
    for selected in selected {
        let checked = row::read(&body, &base, capture, selected, year)?;
        if rows.iter().any(|existing: &types::Row| {
            existing.participant.athlete_id == checked.participant.athlete_id
                && existing.calendar.meet_id == checked.calendar.meet_id
                && existing.context.handle == checked.context.handle
                && existing.context.gender == checked.context.gender
        }) {
            return Err(invalid("duplicate selected TFRRS native performance"));
        }
        rows.push(checked);
    }
    let context = serialized_digest(&(
        "retained-tfrrs-list-context/v1",
        capture.requested_url(),
        capture.body_sha256(),
        &list_id,
        year,
        sport,
    ))?;
    Ok(CheckedRetainedTfrrsListV1::new(
        capture, body, list_id, year, sport, context, rows,
    ))
}

fn invalid(detail: &'static str) -> RetainedResultOwnershipError {
    RetainedResultOwnershipError::Invalid(detail)
}

fn bounded(needed: usize, available: usize) -> Result<()> {
    if needed > available {
        return Err(RetainedResultOwnershipError::Capacity { needed, available });
    }
    Ok(())
}

#[derive(Clone)]
struct Context {
    handle: u32,
    gender: Gender,
    label: String,
    section_span: Range<usize>,
    header_span: Range<usize>,
}

struct RawRow {
    locator: SourceRowLocator,
    context: Context,
}

#[cfg(test)]
mod tests;
