use crate::model::{Candidate, ModelDecision};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum AiCacheValue {
    Candidate(Candidate),
    Decision(ModelDecision),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCacheRecord {
    pub key: String,
    pub value: AiCacheValue,
    pub completed_at_unix: u64,
}

impl AiCacheRecord {
    pub fn candidate(key: String, candidate: Candidate) -> Self {
        Self {
            key,
            value: AiCacheValue::Candidate(candidate),
            completed_at_unix: unix_time(),
        }
    }

    pub fn decision(key: String, decision: ModelDecision) -> Self {
        Self {
            key,
            value: AiCacheValue::Decision(decision),
            completed_at_unix: unix_time(),
        }
    }

    pub fn candidate_value(&self) -> Option<&Candidate> {
        match &self.value {
            AiCacheValue::Candidate(candidate) => Some(candidate),
            AiCacheValue::Decision(_) => None,
        }
    }

    pub fn decision_value(&self) -> Option<&ModelDecision> {
        match &self.value {
            AiCacheValue::Candidate(_) => None,
            AiCacheValue::Decision(decision) => Some(decision),
        }
    }
}

pub fn extraction_key(
    source_key: &str,
    candidate_url: &str,
    evidence: &str,
    model: &str,
    schema_version: u32,
) -> String {
    fingerprint(&[
        "candidate-extraction",
        source_key,
        candidate_url,
        evidence,
        model,
        &schema_version.to_string(),
    ])
}

pub fn decision_key(
    source_key: &str,
    candidate_evidence: &str,
    model: &str,
    schema_version: u32,
) -> String {
    fingerprint(&[
        "identity-decision",
        source_key,
        candidate_evidence,
        model,
        &schema_version.to_string(),
    ])
}

pub fn fingerprint(parts: &[&str]) -> String {
    let digest = parts.iter().fold(Sha256::new(), |mut digest, part| {
        digest.update(
            u64::try_from(part.len())
                .map_or(u64::MAX, |value| value)
                .to_be_bytes(),
        );
        digest.update(part.as_bytes());
        digest
    });
    format!("{:x}", digest.finalize())
}

pub fn load_latest(path: &Path) -> Result<HashMap<String, AiCacheRecord>> {
    crate::jsonl::load(path, |record: &AiCacheRecord| record.key.clone())
}

pub fn append(path: &Path, record: &AiCacheRecord) -> Result<()> {
    crate::jsonl::append(path, record)
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Candidate, ModelDecision};
    use anyhow::Context;
    use tempfile::tempdir;

    #[test]
    fn extraction_key_changes_with_row_evidence_model_and_schema() {
        let base = extraction_key(
            "Export:2",
            "https://www.athletic.net/athlete/7",
            "evidence",
            "model-a",
            1,
        );
        assert_ne!(
            base,
            extraction_key(
                "Export:3",
                "https://www.athletic.net/athlete/7",
                "evidence",
                "model-a",
                1,
            )
        );
        assert_ne!(
            base,
            extraction_key(
                "Export:2",
                "https://www.athletic.net/athlete/7",
                "changed",
                "model-a",
                1,
            )
        );
        assert_ne!(
            base,
            extraction_key(
                "Export:2",
                "https://www.athletic.net/athlete/7",
                "evidence",
                "model-b",
                1,
            )
        );
        assert_ne!(
            base,
            extraction_key(
                "Export:2",
                "https://www.athletic.net/athlete/7",
                "evidence",
                "model-a",
                2,
            )
        );
    }

    #[test]
    fn cache_round_trips_candidate_and_decision() -> Result<()> {
        let directory = tempdir()?;
        let path = directory.path().join("ai-cache.jsonl");
        append(
            &path,
            &AiCacheRecord::candidate("candidate-key".to_owned(), Candidate::default()),
        )?;
        append(
            &path,
            &AiCacheRecord::decision("decision-key".to_owned(), ModelDecision::default()),
        )?;
        let records = load_latest(&path)?;
        let candidate = records
            .get("candidate-key")
            .context("missing candidate cache entry")?;
        let decision = records
            .get("decision-key")
            .context("missing decision cache entry")?;
        assert!(matches!(candidate.value, AiCacheValue::Candidate(_)));
        assert!(matches!(decision.value, AiCacheValue::Decision(_)));
        Ok(())
    }
}
