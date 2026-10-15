use crate::public_identity::{context::Inputs, PublicIdentityError};
use crate::{StoreError, StoreResult};
use census_domain::model::{
    serialized_digest, AthleteCandidateId, CanonicalAthlete, SourceObservation,
};
use census_domain::public_identity_history::local_adjudication::{
    RetainedClaimV1, RetainedIdentitySectionV1,
};
use serde_json::Value;

pub(super) fn inventory(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
) -> StoreResult<Vec<RetainedClaimV1>> {
    let mut inventory = Inventory {
        claims: Vec::new(),
        remaining: super::MAX_WITNESS_BYTES,
    };
    let mut primaries = std::collections::BTreeSet::new();
    for member in members {
        let (ordinal, athlete) = inputs
            .athletes
            .iter()
            .enumerate()
            .find(|(_, row)| row.id.as_str() == member.as_str())
            .ok_or(PublicIdentityError::Corrupt("manual member missing"))?;
        if let Some(primary) = &athlete.source {
            let key = serialized_digest(&(&primary.namespace, &primary.id))?;
            if !primaries.insert(key) {
                return Err(PublicIdentityError::Corrupt(
                    "manual correction needs source epoch evidence",
                )
                .into());
            }
        }
        inventory.athlete(member, ordinal, athlete)?;
        for (ordinal, observation) in inputs.observations.iter().enumerate() {
            let SourceObservation::Athlete(row) = observation else {
                continue;
            };
            if athlete.identities().any(|source| {
                source.namespace == row.namespace && source.id == row.source_athlete_id
            }) {
                let value = encoded_value(observation)?;
                inventory.push(
                    member,
                    RetainedIdentitySectionV1::SourceObservation,
                    ordinal,
                    &serialized_digest(observation)?,
                    String::new(),
                    &value,
                )?;
            }
        }
    }
    Ok(inventory.claims)
}

struct Inventory {
    claims: Vec<RetainedClaimV1>,
    remaining: usize,
}

impl Inventory {
    fn athlete(
        &mut self,
        member: &AthleteCandidateId,
        ordinal: usize,
        athlete: &CanonicalAthlete,
    ) -> StoreResult<()> {
        let value = encoded_value(athlete)?;
        let row_digest = serialized_digest(athlete)?;
        let fields = value
            .as_object()
            .ok_or(PublicIdentityError::Corrupt("manual athlete record"))?;
        for (field, value) in fields {
            match value {
                Value::Array(values) if !values.is_empty() => {
                    for (index, value) in values.iter().enumerate() {
                        self.push(
                            member,
                            RetainedIdentitySectionV1::Athlete,
                            ordinal,
                            &row_digest,
                            format!("/{field}/{index}"),
                            value,
                        )?;
                    }
                }
                _ => self.push(
                    member,
                    RetainedIdentitySectionV1::Athlete,
                    ordinal,
                    &row_digest,
                    format!("/{field}"),
                    value,
                )?,
            }
        }
        Ok(())
    }

    fn push(
        &mut self,
        candidate: &AthleteCandidateId,
        section: RetainedIdentitySectionV1,
        ordinal: usize,
        row_digest: &str,
        claim_path: String,
        value: &Value,
    ) -> StoreResult<()> {
        if self.claims.len() >= 4096 {
            return Err(
                PublicIdentityError::Corrupt("manual claim inventory exceeds bound").into(),
            );
        }
        let claim = RetainedClaimV1 {
            candidate: candidate.clone(),
            section,
            ordinal: u64::try_from(ordinal).map_err(|_| StoreError::CounterOverflow)?,
            row_digest: row_digest.to_owned(),
            claim_path,
            claim_digest: serialized_digest(value)?,
            retained_value: value.clone(),
        };
        let encoded = crate::public_identity::codec::encode(&claim, self.remaining)?;
        self.remaining = self
            .remaining
            .checked_sub(encoded.len())
            .ok_or(StoreError::CounterOverflow)?;
        self.claims
            .try_reserve(1)
            .map_err(|error| StoreError::Refused {
                detail: format!("manual claim inventory allocation: {error}"),
            })?;
        self.claims.push(claim);
        Ok(())
    }
}

fn encoded_value<T: serde::Serialize>(row: &T) -> StoreResult<Value> {
    serde_json::to_value(row).map_err(|source| StoreError::Json {
        detail: "encoding retained manual claim".into(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_identity_tests::support::{subject, TestResult};

    #[test]
    fn running_claim_budget_accepts_exact_size_and_stops_before_retaining_overflow() -> TestResult {
        let candidate = subject("claim-budget", "1")?.id.cast();
        let value = serde_json::json!("retained observation");
        let claim = RetainedClaimV1 {
            candidate,
            section: RetainedIdentitySectionV1::Athlete,
            ordinal: 0,
            row_digest: "row-digest".into(),
            claim_path: "/name".into(),
            claim_digest: serialized_digest(&value)?,
            retained_value: value.clone(),
        };
        let size = serde_json::to_vec(&claim)?.len();
        let mut inventory = Inventory {
            claims: Vec::new(),
            remaining: size,
        };
        inventory.push(
            &claim.candidate,
            claim.section,
            0,
            &claim.row_digest,
            claim.claim_path.clone(),
            &value,
        )?;
        assert_eq!(inventory.remaining, 0);
        assert_eq!(inventory.claims, vec![claim.clone()]);
        assert!(inventory
            .push(
                &claim.candidate,
                claim.section,
                0,
                &claim.row_digest,
                claim.claim_path.clone(),
                &value
            )
            .is_err());
        assert_eq!(inventory.claims, vec![claim.clone()]);
        let mut short = Inventory {
            claims: Vec::new(),
            remaining: size.checked_sub(1).ok_or("size")?,
        };
        assert!(short
            .push(
                &claim.candidate,
                claim.section,
                0,
                &claim.row_digest,
                claim.claim_path.clone(),
                &value
            )
            .is_err());
        assert!(short.claims.is_empty());
        Ok(())
    }
}
