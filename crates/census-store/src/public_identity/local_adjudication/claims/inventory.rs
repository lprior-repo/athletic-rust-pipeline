use super::*;

pub(in crate::public_identity) struct Inventory {
    pub(in crate::public_identity) claims: Vec<RetainedClaimV1>,
    pub(in crate::public_identity) remaining: usize,
}

impl Inventory {
    pub(in crate::public_identity) fn observation(
        &mut self,
        member: &AthleteCandidateId,
        athlete: &CanonicalAthlete,
        ordinal: usize,
        observation: &SourceObservation,
    ) -> StoreResult<()> {
        let SourceObservation::Athlete(row) = observation else {
            return Ok(());
        };
        if athlete
            .identities()
            .any(|source| source.namespace == row.namespace && source.id == row.source_athlete_id)
        {
            let value = encoded_value(observation)?;
            self.push(
                member,
                RetainedIdentitySectionV1::SourceObservation,
                ordinal,
                &serialized_digest(observation)?,
                String::new(),
                &value,
            )?;
        }
        Ok(())
    }

    pub(in crate::public_identity) fn athlete(
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

    pub(in crate::public_identity) fn push(
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

pub(super) fn encoded_value<T: serde::Serialize>(row: &T) -> StoreResult<Value> {
    serde_json::to_value(row).map_err(|source| StoreError::Json {
        detail: "encoding retained manual claim".into(),
        source,
    })
}
