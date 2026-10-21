use super::{hash, invalid, MAX_PAGES};
use crate::CrawlResult;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MhsaaCoopTextLocatorV1 {
    page: u16,
    range: Range<usize>,
    sha256: String,
}

impl MhsaaCoopTextLocatorV1 {
    pub(super) fn new(text: &str, page: u16, range: Range<usize>) -> CrawlResult<Self> {
        if page == 0 || usize::from(page) > MAX_PAGES || range.is_empty() {
            return Err(invalid(
                "MHSAA co-op text locator is empty or outside bounds",
            ));
        }
        let raw = text
            .get(range.clone())
            .ok_or_else(|| invalid("MHSAA co-op text locator is outside UTF-8 text"))?;
        Ok(Self {
            page,
            range,
            sha256: hash(raw.as_bytes()),
        })
    }

    pub fn page(&self) -> u16 {
        self.page
    }

    pub fn byte_offset(&self) -> usize {
        self.range.start
    }

    pub fn byte_length(&self) -> usize {
        self.range.len()
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

pub(super) fn literal(
    text: &str,
    page: u16,
    range: Range<usize>,
    needle: &str,
) -> CrawlResult<MhsaaCoopTextLocatorV1> {
    let raw = text
        .get(range.clone())
        .ok_or_else(|| invalid("MHSAA co-op literal container is outside text"))?;
    let mut matches = raw.match_indices(needle);
    let (offset, _) = matches
        .next()
        .ok_or_else(|| invalid("MHSAA co-op literal is absent"))?;
    if needle.is_empty() || matches.next().is_some() {
        return Err(invalid("MHSAA co-op literal is ambiguous"));
    }
    let start = range
        .start
        .checked_add(offset)
        .ok_or_else(|| invalid("MHSAA co-op literal offset overflow"))?;
    let end = start
        .checked_add(needle.len())
        .ok_or_else(|| invalid("MHSAA co-op literal length overflow"))?;
    MhsaaCoopTextLocatorV1::new(text, page, start..end)
}
