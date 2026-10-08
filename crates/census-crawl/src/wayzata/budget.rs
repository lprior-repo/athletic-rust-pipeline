use crate::{CrawlError, CrawlResult};

pub(super) const MAX_YEARS: usize = 100;
pub(super) const MAX_PAGE_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_PAGE_ROWS: usize = 4096;
pub(super) const MAX_ROW_BYTES: usize = 64 * 1024;
pub(super) const MAX_FIELD_BYTES: usize = 4096;
pub(super) const MAX_UNFINISHED: usize = 100_000;

pub(super) fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

pub(super) fn check(
    resource_name: &'static str,
    requested: usize,
    limit: usize,
) -> CrawlResult<()> {
    if requested > limit {
        return Err(resource(resource_name, requested, limit));
    }
    Ok(())
}

pub(super) fn reserve<T>(values: &mut Vec<T>, extra: usize, limit: usize) -> CrawlResult<()> {
    let requested = values
        .len()
        .checked_add(extra)
        .ok_or_else(|| arithmetic("Wayzata capacity"))?;
    check("Wayzata collection", requested, limit)?;
    values
        .try_reserve(extra)
        .map_err(|_| resource("Wayzata allocation", requested, limit))
}

pub(super) fn add(left: u64, right: u64) -> CrawlResult<u64> {
    left.checked_add(right)
        .ok_or_else(|| arithmetic("Wayzata counter"))
}

pub(super) fn delta(after: u64, before: u64) -> CrawlResult<u64> {
    after
        .checked_sub(before)
        .ok_or_else(|| arithmetic("Wayzata transport counter decreased"))
}

pub(super) fn arithmetic(detail: &str) -> CrawlError {
    CrawlError::Arithmetic {
        detail: detail.to_string(),
    }
}

struct Detail {
    text: String,
    characters: usize,
}

impl std::fmt::Write for Detail {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let remaining = 4096usize
            .checked_sub(self.characters)
            .ok_or(std::fmt::Error)?;
        let end = value
            .char_indices()
            .nth(remaining)
            .map_or(value.len(), |(index, _)| index);
        let prefix = value.get(..end).ok_or(std::fmt::Error)?;
        self.characters = self
            .characters
            .checked_add(prefix.chars().count())
            .ok_or(std::fmt::Error)?;
        self.text.push_str(prefix);
        Ok(())
    }
}

pub(super) fn detail(error: &CrawlError) -> CrawlResult<String> {
    let mut output = Detail {
        text: String::new(),
        characters: 0,
    };
    output
        .text
        .try_reserve(16384)
        .map_err(|_| resource("Wayzata error detail", 16384, 16384))?;
    std::fmt::write(&mut output, format_args!("{error}"))
        .map_err(|_| arithmetic("Wayzata error detail formatting"))?;
    Ok(output.text)
}
