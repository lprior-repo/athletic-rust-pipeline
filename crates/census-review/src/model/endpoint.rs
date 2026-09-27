use super::ModelError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedEndpoint {
    url: String,
}

impl ValidatedEndpoint {
    pub(super) fn new(raw: &str) -> Result<Self, ModelError> {
        let url = url::Url::parse(raw).map_err(|_| ModelError::InvalidEndpoint {
            reason: "invalid URL",
        })?;

        if url.scheme() != "http" {
            return Err(ModelError::InvalidEndpoint {
                reason: "only http scheme is allowed",
            });
        }

        if raw.contains('?') || raw.contains('#') {
            return Err(ModelError::InvalidEndpoint {
                reason: "query strings and fragments not allowed",
            });
        }

        if raw.bytes().any(|b| b < 0x20 || b == b'\\') {
            return Err(ModelError::InvalidEndpoint {
                reason: "control characters not allowed",
            });
        }

        let host = url.host().ok_or(ModelError::InvalidEndpoint {
            reason: "endpoint must include a host",
        })?;

        let loopback = match host {
            url::Host::Ipv4(addr) => addr.is_loopback(),
            url::Host::Ipv6(addr) => addr.is_loopback(),
            _ => {
                return Err(ModelError::InvalidEndpoint {
                    reason: "only IP addresses are allowed",
                });
            }
        };

        if !loopback {
            return Err(ModelError::InvalidEndpoint {
                reason: "only loopback addresses are allowed",
            });
        }

        let port = url.port().ok_or(ModelError::InvalidEndpoint {
            reason: "port must be explicit",
        })?;
        if port == 0 {
            return Err(ModelError::InvalidEndpoint {
                reason: "port must be between 1 and 65535",
            });
        }

        let path = url.path();
        if path.contains("..") || path.contains("./") || path.contains("/.") {
            return Err(ModelError::InvalidEndpoint {
                reason: "dot-segments in path not allowed",
            });
        }
        if path.bytes().any(|b| b == b'%') {
            return Err(ModelError::InvalidEndpoint {
                reason: "percent-encoded path segments not allowed",
            });
        }

        if path != "/" {
            return Err(ModelError::InvalidEndpoint {
                reason: "only root path is allowed",
            });
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(ModelError::InvalidEndpoint {
                reason: "userinfo not allowed",
            });
        }

        Ok(Self {
            url: raw.trim_end_matches('/').to_string(),
        })
    }

    pub(super) fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.url)
    }
}
