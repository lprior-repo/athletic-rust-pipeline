use super::error::{ModelEndpointFailure, ModelError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub(super) struct ValidatedEndpoint {
    url: String,
}

impl ValidatedEndpoint {
    pub(super) fn new(raw: &str) -> Result<Self, ModelError> {
        validate(raw)?;
        Ok(Self {
            url: raw.trim_end_matches('/').to_string(),
        })
    }

    pub(super) fn from_owned(mut raw: String) -> Result<Self, ModelError> {
        validate(&raw)?;
        raw.truncate(raw.trim_end_matches('/').len());
        Ok(Self { url: raw })
    }

    pub(super) fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.url)
    }
}

fn validate(raw: &str) -> Result<(), ModelError> {
    let url = url::Url::parse(raw).map_err(|_| invalid(ModelEndpointFailure::InvalidUrl))?;
    check_scheme(url.scheme())?;
    check_text(raw)?;
    check_host(&url)?;
    check_port(&url)?;
    check_path(&url)?;
    check_userinfo(&url)
}

fn invalid(reason: ModelEndpointFailure) -> ModelError {
    ModelError::InvalidEndpoint { reason }
}

fn check_scheme(scheme: &str) -> Result<(), ModelError> {
    if scheme != "http" {
        return Err(invalid(ModelEndpointFailure::HttpRequired));
    }
    Ok(())
}

fn check_text(raw: &str) -> Result<(), ModelError> {
    if raw.contains('?') || raw.contains('#') {
        return Err(invalid(ModelEndpointFailure::QueryOrFragment));
    }
    if raw.bytes().any(|byte| byte < 0x20 || byte == b'\\') {
        return Err(invalid(ModelEndpointFailure::ControlCharacters));
    }
    Ok(())
}

fn check_host(url: &url::Url) -> Result<(), ModelError> {
    let loopback = match url
        .host()
        .ok_or_else(|| invalid(ModelEndpointFailure::MissingHost))?
    {
        url::Host::Ipv4(address) => address.is_loopback(),
        url::Host::Ipv6(address) => address.is_loopback(),
        _ => return Err(invalid(ModelEndpointFailure::IpRequired)),
    };
    if !loopback {
        return Err(invalid(ModelEndpointFailure::LoopbackRequired));
    }
    Ok(())
}

fn check_port(url: &url::Url) -> Result<(), ModelError> {
    let port = url
        .port()
        .ok_or_else(|| invalid(ModelEndpointFailure::ExplicitPortRequired))?;
    if port == 0 {
        return Err(invalid(ModelEndpointFailure::NonZeroPortRequired));
    }
    Ok(())
}

fn check_userinfo(url: &url::Url) -> Result<(), ModelError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid(ModelEndpointFailure::UserInfo));
    }
    Ok(())
}

fn check_path(url: &url::Url) -> Result<(), ModelError> {
    let path = url.path();
    if path.contains("..") || path.contains("./") || path.contains("/.") {
        return Err(invalid(ModelEndpointFailure::DotSegments));
    }
    if path.bytes().any(|byte| byte == b'%') {
        return Err(invalid(ModelEndpointFailure::EncodedPath));
    }
    if path != "/" {
        return Err(invalid(ModelEndpointFailure::RootPathRequired));
    }
    Ok(())
}
