use super::{cells, invalid, types, Result};
use crate::participation_evidence::result_ownership::tfrrs::{marks, native_links};
use url::Url;

pub(super) fn read(
    body: &str,
    cells: &[cells::Cell],
    base: &Url,
    calendar: &types::Calendar,
    event: &str,
) -> Result<types::Performance> {
    let cell = cells
        .iter()
        .find(|cell| cell.label == "Time" || cell.label == "Mark")
        .ok_or_else(|| invalid("TFRRS row mark cell"))?;
    let link = native_links::single(super::cell_html(body, cell)?, cell.span.start, base)?;
    let id = native_links::performance(&link.url, &calendar.meet_id, &calendar.meet_slug, event)?;
    let mark = marks::read(body, cell, &link)?;
    for cell in cells
        .iter()
        .filter(|cell| cell.label == "Conv" || cell.label == "Wind")
    {
        if !super::cell_text(body, cell)?.is_empty() {
            return Err(invalid("unsupported TFRRS auxiliary conversion or wind"));
        }
    }
    Ok(types::Performance {
        id,
        path: link.url.path().into(),
        mark,
    })
}
