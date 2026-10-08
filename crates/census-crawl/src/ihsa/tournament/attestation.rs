use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    AttestationQualification, IdentityAttestation, SourceIdentity, SourceRef,
};

#[derive(Clone)]
pub(super) struct CaptureLineage {
    url: String,
    digest: String,
    acquired_at: String,
}

impl CaptureLineage {
    pub(super) fn from_capture(capture: FetchOutcome) -> CrawlResult<Self> {
        if capture.status != 200
            || capture.bytes != capture.body.len()
            || !capture_metadata(&capture)
        {
            return Err(CrawlError::Schema {
                url: capture.url,
                detail: "identity capture lacks bound immutable bytes or acquisition metadata"
                    .into(),
            });
        }
        Ok(Self {
            url: capture.url,
            digest: capture.content_digest,
            acquired_at: capture.fetched_at,
        })
    }

    pub(super) fn acquired_at(&self) -> &str {
        &self.acquired_at
    }

    pub(super) fn claim(&self, subject: &SourceIdentity, locator: &str) -> IdentityAttestation {
        IdentityAttestation {
            subject: subject.clone(),
            source: SourceRef::new("ihsa", Some(self.url.clone())),
            capture_sha256: self.digest.clone(),
            acquired_at: self.acquired_at.clone(),
            source_family: "ihsa".into(),
            upstream_producer: "unqualified:ihsa-state-feed".into(),
            subject_locator: locator.to_owned(),
            qualification: AttestationQualification::CandidateOnly,
        }
    }

    pub(super) fn matches(&self, claim: &IdentityAttestation) -> bool {
        claim.capture_sha256 == self.digest
            && claim.source.url.as_deref() == Some(self.url.as_str())
            && claim.acquired_at == self.acquired_at
    }
}

fn capture_metadata(capture: &FetchOutcome) -> bool {
    capture.url.len() <= 4096
        && !capture.url.chars().any(char::is_whitespace)
        && capture.content_digest == crate::net::cache::content_digest(&capture.body)
        && capture.content_digest.len() == 64
        && capture
            .content_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && capture.fetched_at.len() <= 64
        && chrono::DateTime::parse_from_rfc3339(&capture.fetched_at).is_ok()
        && url::Url::parse(&capture.url).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
}
