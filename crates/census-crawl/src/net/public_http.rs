use super::{FetchError, FetchOptions, FetchOutcome, Fetcher, REQUEST_TIMEOUT_SECS};
use std::time::Duration;
use url::{Origin, Url};

#[derive(Clone, Copy)]
pub(in crate::net) enum TransportScope<'a> {
    Provider,
    PublicHttp(&'a Origin),
}

impl TransportScope<'_> {
    pub(in crate::net) fn check(self, raw: &str) -> Result<(), FetchError> {
        if let Self::PublicHttp(origin) = self {
            let target = parse_url(raw)?;
            if &target.origin() != origin {
                return Err(refusal("public-source request left its authorized origin"));
            }
            if !target.username().is_empty() || target.password().is_some() {
                return Err(refusal("public-source URLs must not contain credentials"));
            }
        }
        Ok(())
    }

    pub(in crate::net) fn accept(self, outcome: FetchOutcome) -> Result<FetchOutcome, FetchError> {
        self.check(
            outcome
                .response_url
                .as_deref()
                .map_or(outcome.url.as_str(), |url| url),
        )?;
        Ok(outcome)
    }
}

impl Fetcher {
    #[tracing::instrument(skip(self, options))]
    pub async fn get_public_http(
        &self,
        url: &str,
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        self.destination.validate_url(url)?;
        let target = parse_url(url)?;
        let host = target
            .host_str()
            .ok_or_else(|| refusal("public-source URL has no host"))?;
        if let Some(reason) = crate::registry::acquisition_refusal_for_host(host) {
            return Err(refusal(reason));
        }
        if crate::registry::transport_for_host(host)
            == Some(crate::registry::TransportKind::Browser)
        {
            return Err(refusal(
                "public-source research requires the public HTTP lane",
            ));
        }
        let origin = target.origin();
        let mut session = super::acquisition::CaptureSession::Legacy;
        let timeout_secs = 180;
        tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.fetch_with_scope(
                "GET",
                url,
                None,
                options,
                REQUEST_TIMEOUT_SECS,
                &mut session,
                TransportScope::PublicHttp(&origin),
            ),
        )
        .await
        .map_err(|_| FetchError::Timeout {
            url: url.to_owned(),
            timeout_secs,
        })?
    }
}

pub(in crate::net) fn parse_url(raw: &str) -> Result<Url, FetchError> {
    Url::parse(raw).map_err(|source| FetchError::InvalidUrl {
        url: raw.to_owned(),
        source,
    })
}

pub(in crate::net) fn refusal(detail: &str) -> FetchError {
    FetchError::Policy {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests;
