use crate::{
    ai_cache::{self, AiCacheRecord},
    config::Config,
    extract::{
        candidate_evidence, candidate_from_evidence_required, validate_model_decision,
        OllamaClient, AI_SCHEMA_VERSION,
    },
    model::{Candidate, ModelDecision, Prospect, SearchHit},
};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct Analysis {
    extractor: OllamaClient,
    reviewer: OllamaClient,
    cache: HashMap<String, AiCacheRecord>,
    cache_path: PathBuf,
    page_text_limit: usize,
}

impl Analysis {
    pub fn new(config: &Config, out_dir: &Path) -> Result<Self> {
        let extractor = OllamaClient::new(&config.ollama)?;
        if !extractor.is_enabled() {
            anyhow::bail!("exhaustive matching requires an enabled extraction model");
        }
        let reviewer_config = config
            .identity_review
            .as_ref()
            .context("exhaustive matching requires identity_review model configuration")?;
        let reviewer = OllamaClient::new(reviewer_config)?;
        if !reviewer.is_enabled() {
            anyhow::bail!("exhaustive matching requires an enabled identity reviewer");
        }
        let cache_path = out_dir.join("ai-cache.jsonl");
        let cache = ai_cache::load_latest(&cache_path)?;
        Ok(Self {
            extractor,
            reviewer,
            cache,
            cache_path,
            page_text_limit: config.retrieval.page_text_limit,
        })
    }

    pub async fn candidate(
        &mut self,
        prospect: &Prospect,
        hit: &SearchHit,
        html: Option<&str>,
    ) -> Result<Candidate> {
        let evidence = candidate_evidence(hit, html, self.page_text_limit);
        let serialized_prospect =
            serde_json::to_string(prospect).context("serializing prospect")?;
        let key = ai_cache::extraction_key(
            &serialized_prospect,
            &hit.url,
            &evidence,
            self.extractor.model_name(),
            AI_SCHEMA_VERSION,
        );
        if let Some(record) = self.cache.get(&key) {
            if let Some(candidate_value) = record.candidate_value() {
                return Ok(candidate_value.clone());
            }
        }
        let candidate = candidate_from_evidence_required(
            prospect,
            hit,
            html,
            &self.extractor,
            self.page_text_limit,
        )
        .await?;
        let record = AiCacheRecord::candidate(key.clone(), candidate.clone());
        ai_cache::append(&self.cache_path, &record).context("appending candidate to cache")?;
        self.cache.insert(key, record);
        Ok(candidate)
    }

    pub async fn decision(
        &mut self,
        prospect: &Prospect,
        candidates: &[Candidate],
    ) -> Result<ModelDecision> {
        let serialized_prospect =
            serde_json::to_string(prospect).context("serializing prospect")?;
        let complete_candidate_evidence =
            serde_json::to_string(candidates).context("serializing candidates")?;
        let key = ai_cache::decision_key(
            &serialized_prospect,
            &complete_candidate_evidence,
            self.reviewer.model_name(),
            AI_SCHEMA_VERSION,
        );
        if let Some(record) = self.cache.get(&key) {
            if let Some(decision_value) = record.decision_value() {
                validate_model_decision(decision_value, candidates)?;
                return Ok(decision_value.clone());
            }
        }
        let decision = self
            .reviewer
            .validate_identity_required(prospect, candidates)
            .await?;
        let record = AiCacheRecord::decision(key.clone(), decision.clone());
        ai_cache::append(&self.cache_path, &record).context("appending decision to cache")?;
        self.cache.insert(key, record);
        Ok(decision)
    }
}
