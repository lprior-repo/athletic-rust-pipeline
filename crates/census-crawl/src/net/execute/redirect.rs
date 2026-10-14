use crate::net::{FetchError, Fetcher};

impl Fetcher {
    pub(super) fn resolve_redirect_target(
        &self,
        current_url: &str,
        original_url: &str,
        response: &reqwest::Response,
    ) -> Result<Option<url::Url>, FetchError> {
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            return Ok(None);
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| FetchError::Policy {
                detail: format!("redirect response at {current_url} has no usable Location header"),
            })?;
        let next = redirect_target(current_url, location)?;
        let original = url::Url::parse(original_url).map_err(|source| FetchError::InvalidUrl {
            url: original_url.to_string(),
            source,
        })?;
        if !self.destination.permits_redirect(&original, &next) {
            return Err(FetchError::Policy {
                detail: format!("redirect from {current_url} to {next} bypasses admission"),
            });
        }
        if authentication_path(&next) {
            return Err(FetchError::Policy {
                detail: format!(
                    "redirect from {current_url} to {next} enters an authentication path, so the requested resource was not served"
                ),
            });
        }
        if next
            .host_str()
            .and_then(crate::registry::transport_for_host)
            == Some(crate::registry::TransportKind::Browser)
        {
            return Err(FetchError::Policy {
                detail: format!("HTTP redirect to browser-transport destination {next}"),
            });
        }
        Ok(Some(next))
    }
}

fn redirect_target(current_url: &str, location: &str) -> Result<url::Url, FetchError> {
    url::Url::parse(current_url)
        .and_then(|base| base.join(location))
        .map_err(|source| FetchError::InvalidUrl {
            url: current_url.to_string(),
            source,
        })
}

fn authentication_path(url: &url::Url) -> bool {
    url.path_segments().into_iter().flatten().any(|segment| {
        matches!(
            segment.to_ascii_lowercase().as_str(),
            "login" | "log-in" | "signin" | "sign-in" | "auth" | "oauth" | "sso" | "wp-login.php"
        )
    })
}
