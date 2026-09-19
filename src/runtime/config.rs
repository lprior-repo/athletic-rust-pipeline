use super::{browser::BrowserSettings, browser_config::RawBrowserConfig};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Live,
    Fixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLane {
    Q5_5090,
    Q4_3090,
}

impl ModelLane {
    pub fn key(self) -> &'static str {
        match self {
            Self::Q5_5090 => "q5-5090",
            Self::Q4_3090 => "q4-3090",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    mode: ExecutionMode,
    storage_dir: PathBuf,
    source_origin: String,
    source_interval_ms: u64,
    request_timeout_seconds: u64,
    cpu_workers: usize,
    row_concurrency: usize,
    q5_url: String,
    q5_model: String,
    q4_url: String,
    #[serde(default)]
    browser: RawBrowserConfig,
    q4_model: String,
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    raw: RawConfig,
    source: Url,
    q5: Url,
    q4: Url,
}

impl WorkerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .context("opening worker configuration")?
            .take(65_537)
            .read_to_end(&mut bytes)
            .context("reading bounded worker configuration")?;
        if bytes.len() > 65_536 {
            bail!("worker configuration exceeds 64 KiB");
        }
        let text = std::str::from_utf8(&bytes).context("worker configuration is not UTF-8")?;
        let raw: RawConfig = toml::from_str(text).context("parsing worker configuration")?;
        Self::validate(raw)
    }

    fn validate(raw: RawConfig) -> Result<Self> {
        validate_limits(&raw)?;
        let source = validated_origin(&raw.source_origin)?;
        let q5 = validated_origin(&raw.q5_url)?;
        let q4 = validated_origin(&raw.q4_url)?;
        validate_source(&source, raw.mode)?;
        validate_local(&q5)?;
        validate_local(&q4)?;
        validate_model(&raw.q5_model)?;
        validate_model(&raw.q4_model)?;
        raw.browser.validate(raw.mode)?;
        Ok(Self {
            raw,
            source,
            q5,
            q4,
        })
    }

    pub fn mode(&self) -> ExecutionMode {
        self.raw.mode
    }
    pub fn storage_dir(&self) -> &Path {
        &self.raw.storage_dir
    }
    pub fn source_origin(&self) -> &Url {
        &self.source
    }
    pub(crate) fn browser_settings(&self) -> BrowserSettings {
        self.raw.browser.settings(
            self.storage_dir(),
            self.source.clone(),
            self.request_timeout(),
        )
    }
    pub fn source_interval(&self) -> Duration {
        Duration::from_millis(self.raw.source_interval_ms)
    }
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.raw.request_timeout_seconds)
    }
    pub fn cpu_workers(&self) -> usize {
        self.raw.cpu_workers
    }
    pub fn row_concurrency(&self) -> usize {
        self.raw.row_concurrency
    }
    pub fn model_origin(&self, lane: ModelLane) -> &Url {
        match lane {
            ModelLane::Q5_5090 => &self.q5,
            ModelLane::Q4_3090 => &self.q4,
        }
    }
    pub fn model_id(&self, lane: ModelLane) -> &str {
        match lane {
            ModelLane::Q5_5090 => &self.raw.q5_model,
            ModelLane::Q4_3090 => &self.raw.q4_model,
        }
    }
}

fn validate_limits(raw: &RawConfig) -> Result<()> {
    if !(1..=32).contains(&raw.cpu_workers) {
        bail!("cpu_workers must be in 1..=32");
    }
    if !(1..=128).contains(&raw.row_concurrency) {
        bail!("row_concurrency must be in 1..=128");
    }
    if !(1..=300).contains(&raw.request_timeout_seconds) {
        bail!("request timeout must be in 1..=300 seconds");
    }
    if raw.source_interval_ms > 3_600_000 {
        bail!("source interval exceeds one hour");
    }
    if !raw.storage_dir.is_absolute() {
        bail!("storage_dir must be absolute");
    }
    Ok(())
}

pub(crate) fn validated_origin(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).context("invalid configured HTTP origin")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        bail!("configured endpoint must be an HTTP origin without credentials or path");
    }
    Ok(url)
}

fn validate_local(url: &Url) -> Result<()> {
    let host = url.host_str().context("local model origin has no host")?;
    let address: IpAddr = host
        .trim_matches(['[', ']'])
        .parse()
        .context("local model origin requires a loopback IP literal")?;
    if !address.is_loopback() {
        bail!("model and fixture endpoints must be loopback-only");
    }
    Ok(())
}

pub(crate) fn validate_source(url: &Url, mode: ExecutionMode) -> Result<()> {
    match mode {
        ExecutionMode::Fixture => validate_local(url),
        ExecutionMode::Live => {
            if url.scheme() != "https"
                || url.host_str() != Some("www.athletic.net")
                || url.port().is_some()
            {
                bail!("live source must be https://www.athletic.net/");
            }
            Ok(())
        }
    }
}

fn validate_model(model: &str) -> Result<()> {
    if model.is_empty() || model.len() > 256 || model.chars().any(char::is_control) {
        bail!("model identifier must be bounded nonempty display text");
    }
    Ok(())
}
