use super::*;

impl Fetcher {
    pub(crate) fn acquisition_request(
        &self,
        url: &str,
        options: &FetchOptions,
    ) -> Result<RequestRepresentationV1, FetchError> {
        self.bound_acquisition_request(url, options, false)
    }

    pub(super) fn observed_options(
        &self,
        options: &FetchOptions,
    ) -> Result<FetchOptions, FetchError> {
        let mut options = options.clone();
        let agent = if self.effective_user_agent == super::super::DEFAULT_USER_AGENT {
            "census-service/0.1"
        } else {
            &self.effective_user_agent
        };
        options.headers =
            super::super::execute::representation::observed_headers(&options.headers, agent)?;
        Ok(options)
    }

    pub fn public_resource_request(
        &self,
        url: &str,
        options: &FetchOptions,
    ) -> Result<RequestRepresentationV1, FetchError> {
        self.bound_acquisition_request(url, &self.observed_options(options)?, true)
    }

    pub(super) fn bound_acquisition_request(
        &self,
        url: &str,
        options: &FetchOptions,
        observed: bool,
    ) -> Result<RequestRepresentationV1, FetchError> {
        self.destination.validate_url(url)?;
        let representation =
            super::super::execute::representation::canonical_request(url, &options.headers)?;
        let agent = representation
            .entries()
            .iter()
            .find(|(name, _)| name == "user-agent")
            .map_or(self.effective_user_agent.as_str(), |(_, value)| {
                value.as_str()
            });
        if observed {
            Ok(RequestRepresentationV1::normalized_get(
                url,
                representation.entries().to_vec(),
                agent,
            )?)
        } else {
            Ok(RequestRepresentationV1::get(
                url,
                representation.entries().to_vec(),
                agent,
            )?)
        }
    }
}

#[cfg(test)]
mod tests;
