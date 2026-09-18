use anyhow::{anyhow, bail, Context, Result};
use reqwest::header::{HeaderValue, COOKIE, USER_AGENT};
use serde::Deserialize;
use std::{
    fmt,
    fs::{self, File},
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

const MAX_SESSION_FILE_BYTES: usize = 16 * 1024;
const MAX_HEADER_BYTES: usize = 8 * 1024;

#[derive(Clone)]
pub(crate) struct SourceSession {
    origin: Url,
    user_agent: HeaderValue,
    cookie_header: HeaderValue,
    expires_at_unix_ms: u64,
}

impl fmt::Debug for SourceSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceSession")
            .field("origin", &self.origin)
            .field("user_agent", &"[REDACTED]")
            .field("cookie_header", &"[REDACTED]")
            .field("expires_at_unix_ms", &self.expires_at_unix_ms)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSourceSession {
    origin: String,
    user_agent: String,
    cookie_header: String,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionUseError {
    Expired,
    OriginMismatch,
}

impl SessionUseError {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Expired => "source session is expired",
            Self::OriginMismatch => "source session origin does not match request origin",
        }
    }
}

impl SourceSession {
    pub(crate) fn load(path: &Path, configured_origin: &Url) -> Result<Self> {
        validate_path(path)?;
        let bytes = read_bounded(path)?;
        let raw: RawSourceSession = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow!("source session file contains invalid JSON"))?;
        let origin = crate::runtime::config::validated_origin(&raw.origin)
            .map_err(|_| anyhow!("source session origin is invalid"))?;
        if !same_origin(&origin, configured_origin) {
            bail!("source session origin differs from configured source origin");
        }
        let user_agent = header_value(&raw.user_agent, "source session user-agent")?;
        let cookie_header = header_value(&raw.cookie_header, "source session cookie header")?;
        ensure_fresh(raw.expires_at_unix_ms)?;
        Ok(Self {
            origin,
            user_agent,
            cookie_header,
            expires_at_unix_ms: raw.expires_at_unix_ms,
        })
    }

    pub(crate) fn authorize(&self, request_url: &Url) -> Result<(), SessionUseError> {
        if !same_origin(&self.origin, request_url) {
            return Err(SessionUseError::OriginMismatch);
        }
        if is_expired(self.expires_at_unix_ms) {
            return Err(SessionUseError::Expired);
        }
        Ok(())
    }

    pub(crate) fn attach(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header(USER_AGENT, self.user_agent.clone())
            .header(COOKIE, self.cookie_header.clone())
    }
}

fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("source session path must be absolute");
    }
    let metadata = fs::symlink_metadata(path).context("reading source session metadata")?;
    if !metadata.file_type().is_file() {
        bail!("source session path must be a regular file");
    }
    ensure_private(&metadata)
}

#[cfg(unix)]
fn ensure_private(metadata: &fs::Metadata) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o077 != 0 {
        bail!("source session file permissions are not private");
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private(_metadata: &fs::Metadata) -> Result<()> {
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_SESSION_FILE_BYTES + 1)
        .map_err(|_| anyhow!("source session size limit is unsupported"))?;
    File::open(path)
        .context("opening source session file")?
        .take(limit)
        .read_to_end(&mut bytes)
        .context("reading bounded source session file")?;
    if bytes.len() > MAX_SESSION_FILE_BYTES {
        bail!("source session file exceeds 16 KiB");
    }
    Ok(bytes)
}

fn header_value(raw: &str, label: &str) -> Result<HeaderValue> {
    if raw.is_empty() || raw.len() > MAX_HEADER_BYTES || raw.chars().any(char::is_control) {
        bail!("{label} is outside bounded header syntax");
    }
    let mut value = HeaderValue::from_str(raw).map_err(|_| anyhow!("{label} is invalid"))?;
    value.set_sensitive(true);
    Ok(value)
}

fn ensure_fresh(expires_at_unix_ms: u64) -> Result<()> {
    let now = unix_now_ms()?;
    if expires_at_unix_ms <= now {
        bail!("source session has expired");
    }
    Ok(())
}

fn is_expired(expires_at_unix_ms: u64) -> bool {
    match unix_now_ms() {
        Ok(now) => expires_at_unix_ms <= now,
        Err(_) => true,
    }
}

fn unix_now_ms() -> Result<u64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| anyhow!("system clock predates Unix epoch"))?;
    u64::try_from(elapsed.as_millis()).map_err(|_| anyhow!("system clock exceeds supported range"))
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host() == right.host()
        && left.port_or_known_default() == right.port_or_known_default()
}
