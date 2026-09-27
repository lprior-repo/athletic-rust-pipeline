
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::model::AthleteCandidateId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaseEvidence {
    facts: BTreeSet<EvidenceFact>,
}

impl CaseEvidence {
    pub fn of<'a>(statements: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            facts: statements
                .into_iter()
                .map(|statement| EvidenceFact::Statement(normalize_statement(statement)))
                .collect(),
        }
    }

    pub fn with_members(
        mut self,
        label: &str,
        ids: impl IntoIterator<Item = AthleteCandidateId>,
    ) -> Self {
        let ids: BTreeSet<AthleteCandidateId> = ids.into_iter().collect();
        if !ids.is_empty() {
            self.facts.insert(EvidenceFact::Members {
                label: label.to_string(),
                ids,
            });
        }
        self
    }

    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        for fact in &self.facts {
            fact.hash_into(&mut hasher);
        }
        let mut digest = String::with_capacity(16);
        for byte in hasher.finalize().iter().take(8) {
            digest.push_str(&format!("{byte:02x}"));
        }
        digest
    }
}

pub const MEMBER_SET_LABEL: &str = "members";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceFact {
    Statement(String),
    Members {
        label: String,
        ids: BTreeSet<AthleteCandidateId>,
    },
}

impl EvidenceFact {
    fn hash_into(&self, hasher: &mut Sha256) {
        match self {
            Self::Statement(text) => {
                hasher.update(b"s\x1f");
                hasher.update(text.as_bytes());
            }
            Self::Members { label, ids } => {
                hasher.update(b"m\x1f");
                hasher.update(label.as_bytes());
                for id in ids {
                    hasher.update(b"\x1f");
                    hasher.update(id.as_str().as_bytes());
                }
            }
        }
        hasher.update([0x1e]);
    }
}

pub fn normalize_statement(statement: &str) -> String {
    let tokens: Vec<String> = statement
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '+' && c != '-')
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect();
    tokens.join(" ")
}
