use super::AthleteCandidateId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEvidenceFact {
    pub source: String,
    pub field: String,
    pub value: String,
}

impl ReviewEvidenceFact {
    pub fn new(
        source: impl Into<String>,
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            field: field.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCaseFact {
    pub case_id: String,
    pub family: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPacket {
    pub subject_id: String,
    pub subject: String,
    pub cases: Vec<ReviewCaseFact>,
    pub evidence: Vec<ReviewEvidenceFact>,
}

impl ReviewPacket {
    pub fn new(subject_id: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            subject_id: subject_id.into(),
            subject: subject.into(),
            cases: Vec::new(),
            evidence: Vec::new(),
        }
    }

    pub fn with_case(mut self, case: ReviewCaseFact) -> Self {
        self.cases.push(case);
        self
    }

    pub fn with_evidence(mut self, fact: ReviewEvidenceFact) -> Self {
        self.evidence.push(fact);
        self
    }

    pub fn case_ids(&self) -> Vec<String> {
        self.cases.iter().map(|case| case.case_id.clone()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdictKind {
    ValueProposed,
    InsufficientEvidence,
}

impl ReviewVerdictKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ValueProposed => "value_proposed",
            Self::InsufficientEvidence => "insufficient_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewVerdict {
    pub case_id: String,
    pub kind: ReviewVerdictKind,
    pub field: Option<String>,
    pub value: Option<String>,
    pub confidence: u8,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictBatch {
    pub subject_id: String,
    pub verdicts: Vec<ReviewVerdict>,
}

impl VerdictBatch {
    pub fn sanitize(self, packet: &ReviewPacket) -> (Vec<ReviewVerdict>, usize) {
        let asked = packet.case_ids();
        let mut admitted: Vec<ReviewVerdict> = Vec::with_capacity(self.verdicts.len());
        let mut dropped = 0_usize;
        for mut verdict in self.verdicts {
            if !asked.contains(&verdict.case_id)
                || admitted.iter().any(|kept| kept.case_id == verdict.case_id)
            {
                dropped = dropped.saturating_add(1);
                continue;
            }
            verdict.confidence = verdict.confidence.min(100);
            if verdict.kind == ReviewVerdictKind::ValueProposed && !verdict.proposes_a_value() {
                verdict.kind = ReviewVerdictKind::InsufficientEvidence;
                verdict.field = None;
                verdict.value = None;
            }
            admitted.push(verdict);
        }
        (admitted, dropped)
    }
}

impl ReviewVerdict {
    pub fn proposes_a_value(&self) -> bool {
        let named = |slot: &Option<String>| {
            slot.as_deref()
                .map(str::trim)
                .is_some_and(|text| !text.is_empty())
        };
        named(&self.field) && named(&self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewVerdictRecord {
    pub id: String,
    pub case_id: String,
    pub subject_id: String,
    pub family: String,
    pub kind: String,
    pub field: String,
    pub value: String,
    pub accepted: bool,
    pub confidence: u8,
    pub rationale: String,
    pub reviewer: String,
    pub observed_at: String,
    #[serde(default)]
    pub member_ids: Vec<AthleteCandidateId>,
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
