use super::{document, parse_event_using, DatePrecision, RawPage};
use crate::milesplit::raw_rows::read_block_with_rows;
use crate::milesplit::SourceRowLocator;
use crate::result_file::ParsedRow;
use crate::CrawlResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProvisionalRawRowLocatorV1 {
    pub(crate) event_index: usize,
    pub(crate) row_index: usize,
    pub(crate) row: SourceRowLocator,
}

impl ProvisionalRawRowLocatorV1 {
    pub(crate) fn new(
        event_index: usize,
        row_index: usize,
        ordinal: u32,
        byte_offset: usize,
        byte_length: usize,
    ) -> Self {
        Self {
            event_index,
            row_index,
            row: SourceRowLocator {
                ordinal,
                byte_offset,
                byte_length,
            },
        }
    }
}

pub(crate) type SuccessfulRowVisitor<'a> =
    dyn FnMut(ProvisionalRawRowLocatorV1, &ParsedRow) -> CrawlResult<()> + 'a;

pub(crate) struct LocatedRawPageV1 {
    pub(crate) page: RawPage,
    pub(crate) published_start: String,
    pub(crate) published_end: Option<String>,
}

pub(crate) fn parse_raw_located_bound(
    html: &str,
    url: &str,
    check: &dyn Fn(&str, &str) -> CrawlResult<()>,
    visit: &mut SuccessfulRowVisitor<'_>,
) -> CrawlResult<LocatedRawPageV1> {
    let event = document::read(html, url, document::Ownership::Bound(check))?;
    let published_start = event.start_date.clone();
    let published_end = event.end_date.clone();
    let page = parse_event_using(
        html,
        url,
        event,
        DatePrecision::Published,
        |block, sport, start| read_block_with_rows(block, sport, start, visit),
    )?;
    Ok(LocatedRawPageV1 {
        page,
        published_start,
        published_end,
    })
}

#[cfg(test)]
mod tests;
