use super::*;

impl ResponseObservationV1 {
    pub fn new_observed(
        status: u16,
        response_url: &str,
        observed_at: &str,
        headers: Vec<(String, String)>,
        observed_http: ObservedHttpResponseV1,
    ) -> Result<Self, CaptureDomainError> {
        let mut result = Self::new(status, response_url, observed_at, headers)?;
        if observed_http.status() != status
            || observed_http.response_url() != response_url
            || observed_http.received_at() != observed_at
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed response binding",
            });
        }
        validate_selected(&result.headers, &observed_http)?;
        result.observed_http = Some(Box::new(observed_http));
        Ok(result)
    }
    pub fn observed_http(&self) -> Option<&ObservedHttpResponseV1> {
        self.observed_http.as_deref()
    }
}

fn validate_selected(
    selected: &[(String, String)],
    observed: &ObservedHttpResponseV1,
) -> Result<(), CaptureDomainError> {
    for name in [
        "content-type",
        "etag",
        "last-modified",
        "location",
        "retry-after",
    ] {
        let mut actual = observed
            .headers()
            .iter()
            .filter(|header| header.name() == name);
        let first = actual.next();
        let expected = selected.iter().find(|(key, _)| key == name);
        if actual.next().is_some()
            || first.map(|header| header.value()) != expected.map(|(_, value)| value.as_bytes())
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed response selected headers",
            });
        }
    }
    if selected.iter().any(|(name, _)| {
        !matches!(
            name.as_str(),
            "content-type" | "etag" | "last-modified" | "location" | "retry-after"
        )
    }) {
        return Err(CaptureDomainError::Invalid {
            field: "observed response selected headers",
        });
    }
    Ok(())
}
