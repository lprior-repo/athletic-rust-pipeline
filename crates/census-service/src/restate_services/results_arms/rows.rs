use census_crawl::{AdapterReport, CollectionDisposition, ResolutionCounters, UnresolvedCounters};
use restate_sdk::prelude::HandlerError;

use super::ResultsSourceRows;
use crate::restate_services::jobs::rows_written;

pub(in crate::restate_services) fn failed_source(
    slug: &str,
    meets: usize,
    error: HandlerError,
) -> ResultsSourceRows {
    let source: &dyn std::error::Error = error.as_ref();
    ResultsSourceRows {
        slug: slug.to_string(),
        meets,
        rows: None,
        disposition: CollectionDisposition::Failed,
        errors: 1,
        withheld: None,
        notes: vec![source.to_string()],
        unfinished: Vec::new(),
        unresolved: None,
        resolution: None,
    }
}

pub(in crate::restate_services) fn source_rows(
    slug: &str,
    meets: usize,
    report: AdapterReport,
) -> Result<ResultsSourceRows, HandlerError> {
    Ok(ResultsSourceRows {
        slug: slug.to_string(),
        meets,
        rows: Some(rows_written(&report)?),
        disposition: report.disposition,
        errors: report.errors,
        withheld: Some(report.rejections),
        notes: report.notes,
        unfinished: report.unfinished,
        unresolved: report.unresolved.or_else(|| {
            report
                .disposition
                .is_complete()
                .then_some(UnresolvedCounters { rows: 0, labels: 0 })
        }),
        resolution: report.resolution.or_else(|| {
            report
                .disposition
                .is_complete()
                .then_some(ResolutionCounters::default())
        }),
    })
}
