use super::{SearchResult, Tally};
use crate::net::{FetchError, FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlResult};

mod checked;

#[derive(Clone, Copy, Debug)]
pub(super) enum PageKind {
    Sports,
    AthleticDirector,
}

pub(super) enum PageFailure {
    Http(FetchOutcome),
    Fetch(FetchError),
    Invalid {
        capture: FetchOutcome,
        detail: String,
    },
}

pub(super) struct SchoolPages {
    pub(super) sports: FetchOutcome,
    pub(super) ad: Result<FetchOutcome, PageFailure>,
}

impl PageFailure {
    pub(super) fn detail(&self) -> String {
        match self {
            Self::Http(capture) => format!("HTTP {} for {}", capture.status, capture.url),
            Self::Fetch(error) => error.to_string(),
            Self::Invalid { capture, detail } => {
                format!("invalid page for {}: {detail}", capture.url)
            }
        }
    }

    pub(super) fn payload(&self) -> serde_json::Value {
        match self {
            Self::Http(capture) => serde_json::json!({
                "kind": "http",
                "status": capture.status,
                "capture": super::super::map::capture_note(capture),
                "detail": self.detail(),
            }),
            Self::Fetch(error) => serde_json::json!({
                "kind": "fetch",
                "retryable": error.retryable(),
                "detail": self.detail(),
            }),
            Self::Invalid { capture, detail } => serde_json::json!({
                "kind": "invalid_page",
                "capture": super::super::map::capture_note(capture),
                "detail": detail,
            }),
        }
    }

    pub(super) fn report(
        &self,
        sr: &SearchResult,
        page: &str,
        report: &mut AdapterReport,
        tally: &mut Tally,
    ) -> CrawlResult<()> {
        super::fail(
            report,
            &if page == "sports" {
                sr.sports_url()
            } else {
                sr.ad_url()
            },
            &self.detail(),
        )?;
        tally.fetch_failures = tally
            .fetch_failures
            .checked_add(1)
            .ok_or_else(super::counter_error)?;
        let not_found = match self {
            Self::Http(capture) => capture.status == 404,
            Self::Fetch(FetchError::Http { status, .. }) => *status == 404,
            Self::Fetch(_) | Self::Invalid { .. } => false,
        };
        if not_found {
            tally.not_found = tally
                .not_found
                .checked_add(1)
                .ok_or_else(super::counter_error)?;
        }
        Ok(())
    }
}

#[tracing::instrument(skip(ctx, sr))]
pub(super) async fn fetch_page(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    kind: PageKind,
) -> Result<FetchOutcome, PageFailure> {
    let url = match kind {
        PageKind::Sports => sr.sports_url(),
        PageKind::AthleticDirector => sr.ad_url(),
    };
    let options = FetchOptions {
        allow_not_found: true,
        ..ctx.fetch_options()
    };
    let capture = match ctx.fetcher.get(&url, &options).await {
        Ok(capture) if capture.status == 200 => capture,
        Ok(capture) => return Err(PageFailure::Http(capture)),
        Err(error) => return Err(PageFailure::Fetch(error)),
    };
    match checked::validate(sr, kind, &capture) {
        Ok(()) => Ok(capture),
        Err(detail) => Err(PageFailure::Invalid { capture, detail }),
    }
}

#[cfg(test)]
mod tests;
