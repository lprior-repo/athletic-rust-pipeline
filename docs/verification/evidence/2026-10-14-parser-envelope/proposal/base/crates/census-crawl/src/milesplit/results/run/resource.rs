use super::{capture, ActiveMeet, Intake, MetadataCapture, Run};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};

impl Run {
    pub(super) fn resource(
        &mut self,
        ctx: &AdapterContext<'_>,
        url: &str,
        error: &CrawlError,
    ) -> CrawlResult<()> {
        let url = url
            .get(..url.len().min(4096))
            .map_or("oversized URL", |url| url);
        let payload = self.resource_payload(url, error, ctx.performance_as_of);
        let prefix = match error {
            CrawlError::OwnedParseResource(_) | CrawlError::OwnedCaptureResource(_) => {
                "partial/resource-v2"
            }
            _ => "partial/resource",
        };
        let key = format!("{prefix}/{}", capture::digest(&payload)?);
        let mut batch = ctx.write_batch();
        let staged = super::super::journal_changed(ctx, &mut batch, &key, &payload)
            .and_then(|()| batch.commit());
        let receipt = match staged {
            Ok(()) => "partial receipt retained",
            Err(CrawlError::Resource { .. }) => {
                "partial receipt cannot fit; unfinished locator retained in report"
            }
            Err(error) => return Err(error),
        };
        self.intake = Intake::Blocked { url: url.into() };
        if matches!(&self.stats.resource_stop, super::super::ResourceStop::Open) {
            self.stats.resource_stop =
                super::super::ResourceStop::Unfinished(self.resource_locator(url, error, receipt));
        }
        self.release_owned();
        self.stats.failure(format!(
            "{url}: {error}; {receipt}; previously admitted effects retained"
        ))
    }

    fn resource_payload(
        &self,
        url: &str,
        error: &CrawlError,
        performance_as_of: chrono::NaiveDate,
    ) -> serde_json::Value {
        let owned = match &self.owned {
            Some(ActiveMeet { meet, .. }) => capture::provenance(&meet.outcome.capture),
            None => match resource_capture(error) {
                Some(captured) => capture::provenance(captured),
                None => serde_json::Value::Null,
            },
        };
        let raw = match &self.metadata_capture {
            MetadataCapture::Acquired(capture) => Some(capture),
            MetadataCapture::Unacquired => None,
        };
        let origin = match error {
            CrawlError::OwnedParseResource(failure) => failure.stable_origin().to_string(),
            CrawlError::OwnedCaptureResource(failure) => failure.stable_origin().to_string(),
            _ => error.to_string(),
        };
        serde_json::json!({
            "source_url": url, "owned_capture": owned, "raw_metadata_capture": raw,
            "performance_as_of": performance_as_of,
            "disposition": "resource_limit", "unfinished": true, "resource_error": origin,
        })
    }
    fn resource_locator(&self, url: &str, error: &CrawlError, receipt: &str) -> String {
        let owned = match &self.owned {
            Some(ActiveMeet { meet, .. }) => meet.outcome.capture.content_digest.as_str(),
            None => resource_capture(error)
                .map_or("unacquired", |capture| capture.content_digest.as_str()),
        };
        let raw = match &self.metadata_capture {
            MetadataCapture::Acquired(capture) => capture
                .get("content_digest")
                .and_then(serde_json::Value::as_str)
                .map_or("unacquired", |digest| digest),
            MetadataCapture::Unacquired => "unacquired",
        };
        format!("{url}; owned capture {owned}; raw metadata capture {raw}; {error}; {receipt}")
    }
}

pub(super) fn resource_capture(error: &CrawlError) -> Option<&FetchOutcome> {
    match error {
        CrawlError::OwnedParseResource(failure) => match failure.retention() {
            crate::milesplit::OwnedCaptureRetention::Captured { capture, .. } => Some(capture),
            crate::milesplit::OwnedCaptureRetention::Unrecorded => None,
        },
        CrawlError::OwnedCaptureResource(failure) => match failure.retention() {
            crate::milesplit::OwnedCaptureRetention::Captured { capture, .. } => Some(capture),
            crate::milesplit::OwnedCaptureRetention::Unrecorded => None,
        },
        _ => None,
    }
}
