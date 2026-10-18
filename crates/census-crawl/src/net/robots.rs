use super::{FetchError, FetchOptions, FetchOutcome, Fetcher, REQUEST_TIMEOUT_SECS};
use std::time::Duration;
use url::Url;

mod policy;
use policy::{parse_url, Policy, MAX_POLICY_AGE_SECONDS};

#[derive(Clone, Copy)]
pub(in crate::net) enum Admission<'a> {
    Unrestricted,
    Checked(&'a Policy),
}

impl Admission<'_> {
    pub(in crate::net) fn accept(self, outcome: FetchOutcome) -> Result<FetchOutcome, FetchError> {
        self.check(
            outcome
                .response_url
                .as_deref()
                .map_or(outcome.url.as_str(), |url| url),
        )?;
        Ok(outcome)
    }

    pub(in crate::net) fn check(self, url: &str) -> Result<(), FetchError> {
        match self {
            Self::Unrestricted => Ok(()),
            Self::Checked(policy) => policy.check(url),
        }
    }
}

impl Fetcher {
    #[tracing::instrument(skip(self, options))]
    pub async fn get_robots_checked(
        &self,
        url: &str,
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        let timeout_secs = 180;
        tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.get_with_robots_policy(url, options),
        )
        .await
        .map_err(|_| FetchError::Timeout {
            url: url.to_owned(),
            timeout_secs,
        })?
    }

    #[tracing::instrument(skip(self, options))]
    async fn get_with_robots_policy(
        &self,
        url: &str,
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        self.destination.validate_url(url)?;
        let target = parse_url(url)?;
        let host = target
            .host_str()
            .ok_or_else(|| policy::refusal("school URL has no host"))?;
        if let Some(reason) = crate::registry::acquisition_refusal_for_host(host) {
            return Err(policy::refusal(reason));
        }
        if crate::registry::transport_for_host(host)
            == Some(crate::registry::TransportKind::Browser)
        {
            return Err(policy::refusal(
                "school robots checks require the public HTTP lane",
            ));
        }
        let capture = self.robots_capture(&target, options).await?;
        let user_agent = options
            .headers
            .iter()
            .find(|(name, _)| name.trim().eq_ignore_ascii_case("user-agent"))
            .map_or(self.effective_user_agent.as_str(), |(_, value)| {
                value.as_str()
            });
        let policy = Policy::from_capture(&target, &capture, user_agent)?;
        policy.check(url)?;
        self.raise_robots_delay(host, policy.delay()).await?;
        let mut session = super::acquisition::CaptureSession::Legacy;
        self.fetch_with_admission(
            "GET",
            url,
            None,
            options,
            REQUEST_TIMEOUT_SECS,
            &mut session,
            Admission::Checked(&policy),
        )
        .await
    }

    #[tracing::instrument(skip(self, options))]
    async fn robots_capture(
        &self,
        target: &Url,
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        let mut robots = target.clone();
        robots.set_path("/robots.txt");
        robots.set_query(None);
        robots.set_fragment(None);
        let mut policy_options = FetchOptions {
            refresh: options.refresh,
            allow_not_found: true,
            headers: options.headers.clone(),
        };
        let capture = self.get(robots.as_str(), &policy_options).await?;
        if policy::capture_is_current(&capture, chrono::Utc::now())? {
            return Ok(capture);
        }
        if self.offline || !capture.from_cache {
            return Err(policy::refusal(&format!(
                "robots capture is older than {MAX_POLICY_AGE_SECONDS} seconds or has a future timestamp"
            )));
        }
        policy_options.refresh = true;
        let capture = self.get(robots.as_str(), &policy_options).await?;
        if !policy::capture_is_current(&capture, chrono::Utc::now())? {
            return Err(policy::refusal("refreshed robots capture is not current"));
        }
        Ok(capture)
    }
}

#[cfg(test)]
mod tests;
