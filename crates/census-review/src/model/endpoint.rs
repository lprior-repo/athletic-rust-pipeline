use super::ModelError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedEndpoint {
    url: String,
}

impl ValidatedEndpoint {
    pub(super) fn new(raw: &str) -> Result<Self, ModelError> {
        let url = url::Url::parse(raw).map_err(|_| invalid("invalid URL"))?;
        check_scheme(url.scheme())?;
        check_text(raw)?;
        check_host(&url)?;
        check_port(&url)?;
        check_path(&url)?;
        check_userinfo(&url)?;
        Ok(Self {
            url: raw.trim_end_matches('/').to_string(),
        })
    }

    pub(super) fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.url)
    }
}

fn invalid(reason: &'static str) -> ModelError {
    ModelError::InvalidEndpoint { reason }
}

fn check_scheme(scheme: &str) -> Result<(), ModelError> {
    if scheme != "http" {
        return Err(invalid("only http scheme is allowed"));
    }
    Ok(())
}

fn check_text(raw: &str) -> Result<(), ModelError> {
    if raw.contains('?') || raw.contains('#') {
        return Err(invalid("query strings and fragments not allowed"));
    }
    if raw.bytes().any(|byte| byte < 0x20 || byte == b'\\') {
        return Err(invalid("control characters not allowed"));
    }
    Ok(())
}

fn check_host(url: &url::Url) -> Result<(), ModelError> {
    let loopback = match url
        .host()
        .ok_or_else(|| invalid("endpoint must include a host"))?
    {
        url::Host::Ipv4(address) => address.is_loopback(),
        url::Host::Ipv6(address) => address.is_loopback(),
        _ => return Err(invalid("only IP addresses are allowed")),
    };
    if !loopback {
        return Err(invalid("only loopback addresses are allowed"));
    }
    Ok(())
}

fn check_port(url: &url::Url) -> Result<(), ModelError> {
    let port = url.port().ok_or_else(|| invalid("port must be explicit"))?;
    if port == 0 {
        return Err(invalid("port must be between 1 and 65535"));
    }
    Ok(())
}

fn check_userinfo(url: &url::Url) -> Result<(), ModelError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("userinfo not allowed"));
    }
    Ok(())
}

fn check_path(url: &url::Url) -> Result<(), ModelError> {
    let path = url.path();
    if path.contains("..") || path.contains("./") || path.contains("/.") {
        return Err(invalid("dot-segments in path not allowed"));
    }
    if path.bytes().any(|byte| byte == b'%') {
        return Err(invalid("percent-encoded path segments not allowed"));
    }
    if path != "/" {
        return Err(invalid("only root path is allowed"));
    }
    Ok(())
}
