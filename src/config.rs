use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub workbook: WorkbookConfig,
    pub discovery: DiscoveryConfig,
    pub retrieval: RetrievalConfig,
    pub ollama: OllamaConfig,
    pub matching: MatchingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkbookConfig {
    pub sports: Vec<String>,
    pub expected_graduation_year: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryConfig {
    pub athletic_search_url: String,
    pub max_candidates: usize,
    pub request_timeout_seconds: u64,
    pub search_delay_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetrievalConfig {
    pub authorized_direct_fetch: bool,
    #[serde(default)]
    pub saved_pages_dir: Option<PathBuf>,
    pub respect_robots_txt: bool,
    pub delay_ms: u64,
    pub user_agent: String,
    pub page_text_limit: usize,
}

fn default_model_api() -> String {
    "ollama".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_model_api")]
    pub api: String,
    pub enabled: bool,
    pub url: String,
    pub model: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchingConfig {
    pub match_threshold: f64,
    pub close_threshold: f64,
    pub review_threshold: f64,
    pub require_corroboration: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading configuration {}", path.display()))?;
        let config: Self = toml::from_str(&raw).context("parsing TOML configuration")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.workbook.sports.is_empty() {
            bail!("workbook.sports must contain at least one sport");
        }
        if self.discovery.max_candidates == 0 || self.discovery.max_candidates > 10 {
            bail!("discovery.max_candidates must be between 1 and 10");
        }
        if self.discovery.search_delay_ms < 500 {
            bail!("discovery.search_delay_ms must be at least 500 ms");
        }
        if self.retrieval.delay_ms < 500 {
            bail!("retrieval.delay_ms must be at least 500 ms");
        }
        if self.matching.review_threshold > self.matching.close_threshold
            || self.matching.close_threshold > self.matching.match_threshold
        {
            bail!("matching thresholds must satisfy review <= close <= match");
        }
        Ok(())
    }
}
