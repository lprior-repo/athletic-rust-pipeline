use super::engine_valid::{MAX_EVIDENCE_DIGEST_BYTES, MAX_LOGICAL_ID_BYTES, MAX_MEMBERS_HINT};
use super::*;
use crate::DomainError;
use std::collections::BTreeSet;

pub fn refuse_identity_loss(decision: &AppliedAthleteIdentity) -> Result<(), DomainError> {
    if decision.policy != ATHLETE_IDENTITY_POLICY {
        return Err(DomainError::Unsupported { field: "policy" });
    }
    if decision.id.is_empty() || decision.id.len() > MAX_LOGICAL_ID_BYTES {
        return Err(DomainError::OutOfRange { field: "id" });
    }
    if decision.members.is_empty() || decision.members.len() > MAX_MEMBERS_HINT {
        return Err(DomainError::OutOfRange { field: "members" });
    }
    if decision.observed_at.is_empty() || decision.observed_at.len() > MAX_EVIDENCE_DIGEST_BYTES {
        return Err(DomainError::OutOfRange {
            field: "observed_at",
        });
    }
    let mut seen = BTreeSet::new();
    for member in &decision.members {
        if member.evidence_digest.is_empty()
            || member.evidence_digest.len() > MAX_EVIDENCE_DIGEST_BYTES
        {
            return Err(DomainError::OutOfRange {
                field: "evidence_digest",
            });
        }
        if !seen.insert(member.subject.as_str()) {
            return Err(DomainError::Unsupported { field: "members" });
        }
    }
    Ok(())
}
