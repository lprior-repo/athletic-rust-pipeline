use super::*;

impl RequestRepresentationV1 {
    pub fn normalized_get(
        original_url: &str,
        representation_headers: Vec<(String, String)>,
        effective_user_agent: &str,
    ) -> Result<Self, CaptureDomainError> {
        headers(&representation_headers, 8)?;
        if representation_headers
            .iter()
            .any(|(_, value)| !super::super::observed_http::valid_header_value(value.as_bytes()))
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed request representation",
            });
        }
        if representation_headers.iter().any(|(name, _)| {
            !matches!(
                name.as_str(),
                "accept" | "accept-language" | "accept-encoding" | "user-agent"
            )
        }) || value(&representation_headers, "accept-encoding") != Some("identity")
            || value(&representation_headers, "user-agent") != Some(effective_user_agent)
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed request representation",
            });
        }
        let baseline_headers = representation_headers
            .iter()
            .filter(|(name, _)| name != "accept-encoding")
            .cloned()
            .collect();
        let mut request = Self::get(original_url, baseline_headers, effective_user_agent)?;
        request.representation_headers = representation_headers;
        request.transport_recipe = TransportRecipe::ReqwestNormalizedObservedHttpV1;
        Ok(request)
    }
}

fn value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}
