use census_crawl::milesplit::{MeetPages, ResultSetOptions};
use census_crawl::{AdapterReport, CollectionDisposition, ResolutionCounters};
use restate_sdk::prelude::HandlerError;

use crate::restate_services::jobs::invariant;

pub(super) fn options(pages: &MeetPages) -> Result<ResultSetOptions, HandlerError> {
    let urls = pages.files.iter().try_fold(Vec::new(), |mut urls, file| {
        if urls.len() >= 4096 {
            return Err(invariant("meet result-set frontier exceeds 4096"));
        }
        urls.try_reserve(1)
            .map_err(|_| invariant("meet result-set allocation"))?;
        urls.push(file.request());
        Ok(urls)
    })?;
    Ok(ResultSetOptions { urls })
}

pub(super) fn record_quarantines(
    report: AdapterReport,
    quarantined: Vec<(String, String)>,
) -> Result<AdapterReport, HandlerError> {
    quarantined
        .into_iter()
        .try_fold(report, |mut report, (url, reason)| {
            report.errors = report
                .errors
                .checked_add(1)
                .ok_or_else(|| invariant("meet-page quarantine counter overflow"))?;
            report.disposition = CollectionDisposition::Partial;
            report
                .unfinished
                .try_reserve(1)
                .map_err(|_| invariant("quarantine locator allocation"))?;
            report.unfinished.push(url.clone());
            report.note(format!("quarantined meet page {url}: {reason}"));
            let counters = report.resolution.unwrap_or(ResolutionCounters::default());
            let quarantined = counters
                .quarantined
                .checked_add(1)
                .ok_or_else(|| invariant("resolution quarantined counter overflow"))?;
            report.resolution = Some(ResolutionCounters {
                quarantined,
                ..counters
            });
            Ok(report)
        })
}
