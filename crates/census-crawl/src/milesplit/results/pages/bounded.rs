use super::{MeetPage, MeetPages};
use crate::net::{FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};
use std::fmt::Write;

mod discovery;

pub(in crate::milesplit) const MAX_FILES: usize = 4096;
const MAX_TEXT: usize = 4096;

pub struct MeetPageLimits {
    allocation_bytes: u64,
}

impl MeetPageLimits {
    pub fn standard() -> Self {
        Self::new(64 * 1024 * 1024)
    }

    pub fn new(allocation_bytes: u64) -> Self {
        Self { allocation_bytes }
    }
}

pub struct BoundedMeetPages {
    pages: CrawlResult<MeetPages>,
    allocation_requested: u64,
}

impl BoundedMeetPages {
    pub fn allocation_requested(&self) -> u64 {
        self.allocation_requested
    }
    pub fn into_parts(self) -> (CrawlResult<MeetPages>, u64) {
        (self.pages, self.allocation_requested)
    }
}

pub(in crate::milesplit) struct Allocation {
    limit: u64,
    requested: u64,
}

impl Allocation {
    pub(in crate::milesplit) fn new(limits: MeetPageLimits) -> Self {
        Self {
            limit: limits.allocation_bytes,
            requested: 0,
        }
    }

    pub(in crate::milesplit) fn text(&mut self, value: &str) -> CrawlResult<String> {
        self.reserve(value.len())?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(value.len())
            .map_err(|_| resource("meet-page allocation", value.len(), value.len()))?;
        owned.push_str(value);
        Ok(owned)
    }

    pub(in crate::milesplit) fn reserve(&mut self, bytes: usize) -> CrawlResult<()> {
        let bytes = u64::try_from(bytes)
            .map_err(|_| resource("meet-page allocation", usize::MAX, usize::MAX))?;
        let next = self
            .requested
            .checked_add(bytes)
            .ok_or_else(|| resource("meet-page allocation", usize::MAX, usize::MAX))?;
        if next > self.limit {
            return Err(resource(
                "meet-page allocation",
                usize::try_from(next).map_or(usize::MAX, |n| n),
                usize::try_from(self.limit).map_or(usize::MAX, |n| n),
            ));
        }
        self.requested = next;
        Ok(())
    }

    pub(in crate::milesplit) fn push<T>(
        &mut self,
        target: &mut Vec<T>,
        value: T,
        maximum: usize,
    ) -> CrawlResult<()> {
        if target.len() >= maximum {
            return Err(resource(
                "meet-page files",
                target.len().saturating_add(1),
                maximum,
            ));
        }
        if target.len() == target.capacity() {
            let capacity = target
                .capacity()
                .checked_mul(2)
                .map_or(1, |n| n.max(1))
                .min(maximum);
            self.reserve(
                capacity
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or_else(|| resource("meet-page allocation", usize::MAX, usize::MAX))?,
            )?;
            let additional = capacity
                .checked_sub(target.len())
                .ok_or_else(|| resource("meet-page allocation", usize::MAX, usize::MAX))?;
            target
                .try_reserve_exact(additional)
                .map_err(|_| resource("meet-page allocation", capacity, maximum))?;
        }
        target.push(value);
        Ok(())
    }
}

pub(in crate::milesplit) fn resource(
    resource: &'static str,
    requested: usize,
    limit: usize,
) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

pub async fn read_meet_pages(
    fetcher: &Fetcher,
    pages: impl IntoIterator<Item = MeetPage>,
    options: &FetchOptions,
    limits: MeetPageLimits,
) -> BoundedMeetPages {
    let mut allocation = Allocation::new(limits);
    let pages = discovery::discover(fetcher, pages, options, &mut allocation).await;
    BoundedMeetPages {
        pages,
        allocation_requested: allocation.requested,
    }
}

fn lossy_body(mut bytes: &[u8], allocation: &mut Allocation) -> CrawlResult<String> {
    let capacity = bytes
        .len()
        .checked_mul(3)
        .ok_or_else(|| resource("meet-page allocation", usize::MAX, usize::MAX))?;
    allocation.reserve(capacity)?;
    let mut text = String::new();
    text.try_reserve_exact(capacity)
        .map_err(|_| resource("meet-page allocation", capacity, capacity))?;
    while !bytes.is_empty() {
        match std::str::from_utf8(bytes) {
            Ok(valid) => {
                text.push_str(valid);
                break;
            }
            Err(error) => {
                let prefix = error.valid_up_to();
                let valid = bytes
                    .get(..prefix)
                    .ok_or_else(|| resource("meet-page UTF8", usize::MAX, bytes.len()))?;
                text.push_str(
                    std::str::from_utf8(valid)
                        .map_err(|_| resource("meet-page UTF8", prefix, bytes.len()))?,
                );
                text.push('\u{fffd}');
                let consumed = match error.error_len() {
                    Some(length) => prefix
                        .checked_add(length)
                        .ok_or_else(|| resource("meet-page UTF8", usize::MAX, bytes.len()))?,
                    None => bytes.len(),
                };
                bytes = bytes
                    .get(consumed..)
                    .ok_or_else(|| resource("meet-page UTF8", consumed, bytes.len()))?;
            }
        }
    }
    Ok(text)
}

fn raw_url(
    file: &crate::milesplit::MeetResultFile,
    base: &str,
    allocation: &mut Allocation,
) -> CrawlResult<String> {
    let base = base.trim_end_matches('/');
    let digits = if file.id == 0 {
        1
    } else {
        usize::try_from(file.id.unsigned_abs().ilog10())
            .map_err(|_| resource("meet-page URL", usize::MAX, MAX_TEXT))?
            .checked_add(1)
            .ok_or_else(|| resource("meet-page URL", usize::MAX, MAX_TEXT))?
    };
    let tail = if file.inline {
        0
    } else {
        digits
            .checked_add(usize::from(file.id < 0))
            .and_then(|n| n.checked_add(5))
            .ok_or_else(|| resource("meet-page URL", usize::MAX, MAX_TEXT))?
    };
    let maximum = base
        .len()
        .checked_add(tail)
        .ok_or_else(|| resource("meet-page URL", usize::MAX, MAX_TEXT))?;
    if maximum > MAX_TEXT {
        return Err(resource("meet-page URL", maximum, MAX_TEXT));
    }
    allocation.reserve(maximum)?;
    let mut result = String::new();
    result
        .try_reserve_exact(maximum)
        .map_err(|_| resource("meet-page allocation", maximum, MAX_TEXT))?;
    result.push_str(base);
    if !file.inline {
        write!(&mut result, "/{}/raw", file.id)
            .map_err(|_| resource("meet-page URL", maximum, MAX_TEXT))?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
