use super::{AssistantEvidenceRef, AssistantVerdict};
use crate::domain::evidence::EvidenceRef;
use crate::runtime::protocol::{ReviewInput, ReviewVerdict};
use anyhow::{anyhow, bail, Result};

const MAX_REASON_BYTES: usize = 4_096;
const MAX_EVIDENCE_REFS: usize = 32;
const MAX_LOCATOR_BYTES: usize = 1_024;

impl AssistantVerdict {
    pub fn validate(self, input: &ReviewInput) -> Result<Self> {
        match &self {
            Self::Select {
                athlete_id,
                reason,
                evidence,
            } => {
                if reason.is_empty() || reason.len() > MAX_REASON_BYTES {
                    bail!("review selection reason exceeds its bound");
                }
                if evidence.is_empty() || evidence.len() > MAX_EVIDENCE_REFS {
                    bail!("review selection evidence references are invalid");
                }
                let candidate = input
                    .candidates
                    .iter()
                    .find(|item| item.athlete_id == *athlete_id)
                    .ok_or_else(|| anyhow!("review selected an unsupplied athlete ID"))?;
                if candidate.eligibility_reasons.is_empty() {
                    bail!("review selected an ineligible athlete");
                }
                let allowed = candidate_refs(candidate);
                if evidence.iter().any(|reference| {
                    reference.locator.is_empty()
                        || reference.locator.len() > MAX_LOCATOR_BYTES
                        || !allowed.iter().any(|allowed_ref| {
                            allowed_ref.document == reference.document
                                && allowed_ref.locator == reference.locator
                        })
                }) {
                    bail!("review selected an invented or unsupported evidence reference");
                }
            }
            Self::Unresolved { reason } if reason.is_empty() || reason.len() > MAX_REASON_BYTES => {
                bail!("review unresolved reason exceeds its bound");
            }
            Self::Unresolved { .. } => {}
        }
        Ok(self)
    }

    pub fn into_protocol(self) -> Result<ReviewVerdict> {
        match self {
            Self::Select {
                athlete_id,
                reason,
                evidence,
            } => Ok(ReviewVerdict::Select {
                athlete_id,
                reason,
                evidence: evidence
                    .into_iter()
                    .map(|reference| EvidenceRef {
                        document: reference.document,
                        locator: reference.locator,
                    })
                    .collect(),
            }),
            Self::Unresolved { reason } => Ok(ReviewVerdict::Unresolved { reason }),
        }
    }
}

fn candidate_refs(
    candidate: &crate::runtime::protocol::ReviewCandidate,
) -> Vec<AssistantEvidenceRef> {
    candidate
        .teams
        .iter()
        .flat_map(|team| {
            std::iter::once(team.name.evidence.clone()).chain(
                team.location
                    .iter()
                    .flat_map(|location| std::iter::once(location.evidence.clone())),
            )
        })
        .chain(
            candidate
                .graduation_years
                .iter()
                .map(|value| value.evidence.clone()),
        )
        .chain(
            candidate
                .issues
                .iter()
                .filter_map(|issue| issue.evidence.clone()),
        )
        .map(|reference| AssistantEvidenceRef {
            document: reference.document,
            locator: reference.locator,
        })
        .collect()
}
