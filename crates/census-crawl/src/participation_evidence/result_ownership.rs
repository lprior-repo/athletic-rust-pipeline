use super::{DocumentSubjectV1, RetainedHtmlCaptureV1};
use crate::milesplit::{parse_raw_located_bound, ProvisionalRawRowLocatorV1, SourceRowLocator};
use census_domain::model::serialized_digest;
use census_domain::participation_candidates::MAX_PARTICIPATION_FACTS;

mod active;
pub(super) mod discovery;
mod school;
mod selection;
pub use school::{CheckedResultSchoolStatementV1, ResultSchoolStatementV1};
mod types;
pub use types::{CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1};

const INTERPRETATION: &str = "retained-milesplit-html-result-row/v1";

type Result<T> = std::result::Result<T, RetainedResultOwnershipError>;

#[derive(Debug, thiserror::Error)]
pub enum RetainedResultOwnershipError {
    #[error("retained HTML result ownership: {0}")]
    Invalid(&'static str),
    #[error("retained HTML result locator capacity: needed {needed}, available {available}")]
    Capacity { needed: usize, available: usize },
    #[error("retained HTML result allocation refused")]
    Allocation,
    #[error("retained HTML result arithmetic overflow")]
    Overflow,
    #[error("retained HTML result capture: {0}")]
    Capture(
        #[from]
        #[source]
        census_store::StoreError,
    ),
    #[error("retained HTML result parser: {0}")]
    Parser(
        #[from]
        #[source]
        crate::CrawlError,
    ),
    #[error("retained HTML result digest: {0}")]
    Digest(
        #[from]
        #[source]
        census_domain::model::CanonicalJsonError,
    ),
}

pub fn read_retained_html_result_rows<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    requested: &[SourceRowLocator],
) -> Result<CheckedRetainedResultDocumentV1<'a>> {
    selection::admit_count(requested.len())?;
    let body = capture.body()?;
    let owned_pre = active::owned_pre(&body)?;
    selection::validate_spans(&body, requested, &owned_pre)?;
    let native = selection::native_document(capture.requested_url())?;
    read_selected(capture, body, native, requested.len(), |locator| {
        Ok(requested.contains(&locator.row))
    })
}

fn read_selected<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    body: String,
    native: (String, String),
    count: usize,
    mut select: impl FnMut(&ProvisionalRawRowLocatorV1) -> crate::CrawlResult<bool>,
) -> Result<CheckedRetainedResultDocumentV1<'a>> {
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(count)
        .map_err(|_| RetainedResultOwnershipError::Allocation)?;
    let parsed = parse_raw_located_bound(
        &body,
        capture.requested_url(),
        &|published, role| selection::require_owner(capture.requested_url(), published, role),
        &mut |locator, _| {
            if select(&locator)? {
                if selected.len() >= count {
                    return Err(crate::CrawlError::Schema {
                        url: capture.requested_url().into(),
                        detail: "duplicate successful locator".into(),
                    });
                }
                selected.push(locator);
            }
            Ok(())
        },
    )?;
    if selected.len() != count {
        return Err(RetainedResultOwnershipError::Invalid(
            "requested row is not an exact successful original row",
        ));
    }
    let context = serialized_digest(&(
        INTERPRETATION,
        capture,
        &parsed.page.meet,
        &parsed.page.skipped,
        &parsed.page.grade_issues,
        &parsed.published_start,
        &parsed.published_end,
    ))?;
    let selected = selection::finish(&body, capture, selected)?;
    Ok(CheckedRetainedResultDocumentV1 {
        capture,
        body,
        page: parsed.page,
        selected,
        context,
        published_start: parsed.published_start,
        published_end: parsed.published_end,
        meet_id: native.0,
        result_document_id: native.1,
    })
}

#[cfg(test)]
mod tests;
