use super::{budget, ResultSetOptions};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};

pub(in crate::milesplit::results) mod frozen;

const URL_BYTES: usize = 4096;
const RETAINED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Frontier {
    urls: Vec<String>,
    bytes: usize,
    remainder: Remainder,
}

#[derive(Default)]
enum Remainder {
    #[default]
    None,
    Unlisted {
        first: usize,
        count: usize,
    },
}

impl Frontier {
    pub(super) fn unfinished(&mut self, ordinal: usize, url: &str) -> CrawlResult<()> {
        let next = self.bytes.checked_add(url.len()).ok_or_else(overflow)?;
        if url.len() > URL_BYTES
            || self.urls.len() >= budget::METADATA_LABELS
            || next > RETAINED_BYTES
        {
            return self.unlisted(ordinal);
        }
        let mut retained = String::new();
        retained
            .try_reserve_exact(url.len())
            .map_err(budget::reserve)?;
        self.urls.try_reserve(1).map_err(budget::reserve)?;
        retained.push_str(url);
        self.urls.push(retained);
        self.bytes = next;
        Ok(())
    }

    fn unlisted(&mut self, ordinal: usize) -> CrawlResult<()> {
        self.remainder = match self.remainder {
            Remainder::None => Remainder::Unlisted {
                first: ordinal,
                count: 1,
            },
            Remainder::Unlisted { first, count } => Remainder::Unlisted {
                first,
                count: count.checked_add(1).ok_or_else(overflow)?,
            },
        };
        Ok(())
    }

    pub(super) fn finish(
        self,
        ctx: &AdapterContext<'_>,
        options: &ResultSetOptions,
        mut report: AdapterReport,
    ) -> CrawlResult<AdapterReport> {
        report.unfinished = self.urls;
        if let Remainder::Unlisted { first, count } = self.remainder {
            append_remainder(ctx, options, &mut report, first, count)?;
        }
        report.finish_frontier();
        Ok(report)
    }
}

fn append_remainder(
    ctx: &AdapterContext<'_>,
    options: &ResultSetOptions,
    report: &mut AdapterReport,
    first: usize,
    count: usize,
) -> CrawlResult<()> {
    report.unfinished.try_reserve(1).map_err(budget::reserve)?;
    match frozen::archive(ctx, options)
        .and_then(|manifest| manifest.locator(first..options.urls.len(), count))
    {
        Ok(locator) => report.unfinished.push(locator),
        Err(error) => {
            report.errors = report.errors.checked_add(1).ok_or_else(overflow)?;
            report.unfinished.push(format!("unpersisted original request ordinals [{first}..{}); exactly {count} unlisted unfinished occurrences; no resumable input locator", options.urls.len()));
            report.note(format!("original request manifest unavailable: {error}"));
        }
    }
    Ok(())
}

fn overflow() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "unfinished result frontier counter overflow".into(),
    }
}
