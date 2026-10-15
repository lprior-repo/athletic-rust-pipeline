pub(in crate::public_identity) mod manual;
mod outcome;
mod verify;

use super::{codec, graph_reference::GraphAuthorityReferenceV1, PublicIdentityError};
use crate::StoreResult;
use census_domain::model::{serialized_digest, AppliedAthleteIdentity, AthleteCandidateId};
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV3;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV2;
use census_domain::public_identity_history::{PreparedHistoryRecord, RegistryHead};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HeldGraphWitness {
    pub version: u32,
    pub operation: String,
    pub request_digest: String,
    pub record_digest: String,
    pub graph: GraphAuthorityReferenceV1,
    pub observed_at: String,
    pub payload: Payload,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Payload {
    Source {
        candidate: AthleteCandidateId,
        decision: Decision,
        public_id: String,
    },
    Reviewed {
        kind: super::correction::request::ReviewedKind,
        case_id: String,
        decision: Decision,
        #[serde(deserialize_with = "introduced_ids")]
        introduced_public_ids: Vec<String>,
    },
    Membership {
        request: super::SourceCandidateMembershipRequestV1,
        decision: Decision,
    },
    Manual {
        actor: LocalActor,
        adjudication: LocalIdentityAdjudicationRecordV3,
    },
    Withholding {
        actor: LocalActor,
        adjudication: LocalMembershipWithholdingRecordV2,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LocalActor {
    LocalOperator,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Decision {
    pub id: String,
    pub digest: String,
}
impl Decision {
    pub(super) fn new(decision: &AppliedAthleteIdentity) -> StoreResult<Self> {
        Ok(Self {
            id: decision.id.clone(),
            digest: serialized_digest(decision)?,
        })
    }
}
impl HeldGraphWitness {
    pub(super) fn decode(key: &str, bytes: &[u8]) -> StoreResult<Self> {
        if bytes.len() > super::local_adjudication::MAX_WITNESS_BYTES {
            return Err(crate::StoreError::RowTooLarge {
                bytes: bytes.len(),
                max: super::local_adjudication::MAX_WITNESS_BYTES,
            });
        }
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }
        let version: Version = codec::decode(key, bytes)?;
        if bytes.len() > 65_536 && !matches!(version.version, 10 | 12) {
            return Err(crate::StoreError::RowTooLarge {
                bytes: bytes.len(),
                max: 65_536,
            });
        }
        let held: Self = codec::decode(key, bytes)?;
        held.validate_shape()?;
        if bytes.len() > 65_536
            && !matches!(
                held.payload,
                Payload::Manual { .. } | Payload::Withholding { .. }
            )
        {
            return Err(crate::StoreError::RowTooLarge {
                bytes: bytes.len(),
                max: 65_536,
            });
        }
        Ok(held)
    }
    pub(super) fn validate_shape(&self) -> StoreResult<()> {
        self.graph.validate()?;
        let expected = match &self.payload {
            Payload::Source { .. } => 8,
            Payload::Reviewed { .. } => 9,
            Payload::Manual { .. } => 10,
            Payload::Membership { .. } => 11,
            Payload::Withholding { .. } => 12,
        };
        if self.version != expected
            || self.operation.is_empty()
            || self.operation.len() > 256
            || self.operation.chars().any(char::is_control)
            || !crate::journal::is_object_id(&self.request_digest)
            || !crate::journal::is_object_id(&self.record_digest)
            || self.observed_at.is_empty()
            || self.observed_at.len() > 256
            || self.observed_at.chars().any(char::is_control)
        {
            return Err(PublicIdentityError::Corrupt("graph witness schema binding").into());
        }
        Ok(())
    }
    pub(super) fn operation(&self) -> &str {
        &self.operation
    }
    pub(super) fn graph(&self) -> &GraphAuthorityReferenceV1 {
        &self.graph
    }
    pub(super) fn observed_at(&self) -> &str {
        &self.observed_at
    }
    pub(super) fn request_digest(&self) -> &str {
        &self.request_digest
    }
    pub(super) fn decision_id(&self) -> Option<&str> {
        self.decision().map(|value| value.id.as_str())
    }
    fn decision(&self) -> Option<&Decision> {
        match &self.payload {
            Payload::Source { decision, .. }
            | Payload::Reviewed { decision, .. }
            | Payload::Membership { decision, .. } => Some(decision),
            _ => None,
        }
    }
    pub(super) fn new(
        operation: &str,
        request_digest: String,
        graph: GraphAuthorityReferenceV1,
        observed_at: &str,
        history: &PreparedHistoryRecord,
        payload: Payload,
    ) -> StoreResult<Self> {
        let version = match &payload {
            Payload::Source { .. } => 8,
            Payload::Reviewed { .. } => 9,
            Payload::Manual { .. } => 10,
            Payload::Membership { .. } => 11,
            Payload::Withholding { .. } => 12,
        };
        let result = Self {
            version,
            operation: operation.into(),
            request_digest,
            graph,
            observed_at: observed_at.into(),
            record_digest: history.uncommitted_record.digest.clone(),
            payload,
        };
        result.validate_shape()?;
        let limit = if matches!(
            result.payload,
            Payload::Manual { .. } | Payload::Withholding { .. }
        ) {
            super::local_adjudication::MAX_WITNESS_BYTES
        } else {
            65_536
        };
        codec::encode(&result, limit)?;
        Ok(result)
    }
}
pub(super) fn decode_head<'de, D: serde::Deserializer<'de>>(
    parser: D,
) -> Result<RegistryHead, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Head {
        sequence: u64,
        digest: String,
    }
    let value = Head::deserialize(parser)?;
    if !crate::journal::is_object_id(&value.digest) {
        return Err(serde::de::Error::custom("invalid registry head"));
    }
    Ok(RegistryHead {
        sequence: value.sequence,
        digest: value.digest,
    })
}

fn introduced_ids<'de, D: serde::Deserializer<'de>>(parser: D) -> Result<Vec<String>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 128 validated public identity identifiers")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let (values, complete) = (0..128).try_fold(
                (Vec::new(), false),
                |(mut values, complete), _| -> Result<_, A::Error> {
                    if complete {
                        return Ok((values, true));
                    }
                    let Some(value) = sequence.next_element::<String>()? else {
                        return Ok((values, true));
                    };
                    census_domain::public_identity_history::PublicAthleteId::parse(&value)
                        .map_err(serde::de::Error::custom)?;
                    values.try_reserve(1).map_err(serde::de::Error::custom)?;
                    values.push(value);
                    Ok((values, false))
                },
            )?;
            if !complete && sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("graph introduced public ID bound"));
            }
            Ok(values)
        }
    }
    parser.deserialize_seq(Visitor)
}
