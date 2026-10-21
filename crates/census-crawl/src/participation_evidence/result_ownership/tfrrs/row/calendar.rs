use super::{cells, invalid, types, Result};
use crate::participation_evidence::result_ownership::tfrrs::native_links;
use crate::tfrrs::retained_date_label;
use url::Url;

pub(super) fn read(
    body: &str,
    cells: &[cells::Cell],
    base: &Url,
    year: i16,
) -> Result<types::Calendar> {
    let meet = super::named(body, cells::get(cells, "Meet")?, base)?;
    let (meet_id, meet_slug) = native_links::meet(&meet.url)?;
    let date_cell = cells::get(cells, "Meet Date")?;
    let date = retained_date_label(super::cell_text(body, date_cell)?)
        .ok_or_else(|| invalid("TFRRS row meet date"))?;
    if date.year != year {
        return Err(invalid("TFRRS row meet date differs from list year"));
    }
    Ok(types::Calendar {
        meet_id,
        meet_slug,
        meet_path: meet.url.path().into(),
        meet_name: meet.text,
        meet_span: meet.span,
        date_label: date.raw,
        date_iso: date.iso,
        year: date.year,
        month: date.month,
        day: date.day,
        date_span: date_cell.span.clone(),
    })
}
