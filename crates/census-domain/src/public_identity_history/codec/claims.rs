use serde::de::{Error, MapAccess};
use serde::{Deserialize, Deserializer};

use crate::model::{AthleteCandidateId, SourceNamespace};

use super::super::{
    BindingClaim, CandidateDecisionReference, CandidateEvidenceKind, EffectiveScope,
    SourceObjectKey,
};
use super::atoms::{
    CandidateNode, EvidenceDigestNode, EvidenceKindNode, OptionalTextNode, TextNode,
};
use super::containers::{ListNode, Members, Meter, Node, NodeSeed};
use super::namespace::NamespaceNode;
use super::object_decoder::{ObjectFields, ObjectSeed};
use super::requests::ScopeNode;

pub(super) struct EvidenceNode;
pub(super) struct BindingNode;
pub(super) struct SourceNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum EvidenceFields {
    ApplicationId,
    EvidenceDigest,
    Kind,
    Members,
}

impl ObjectFields for EvidenceNode {
    type Field = EvidenceFields;
    type State = (
        Option<String>,
        Option<String>,
        Option<CandidateEvidenceKind>,
        Option<Vec<AthleteCandidateId>>,
    );
    const FIELD_COUNT: usize = 4;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            EvidenceFields::ApplicationId => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("application_id"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<TextNode>::new(meter))?);
            }
            EvidenceFields::EvidenceDigest => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("evidence_digest"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<EvidenceDigestNode>::new(meter))?);
            }
            EvidenceFields::Kind => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("kind"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<EvidenceKindNode>::new(meter))?);
            }
            EvidenceFields::Members => {
                if state.3.is_some() {
                    return Err(A::Error::duplicate_field("members"));
                }
                state.3 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<CandidateNode, Members>>::new(meter))?,
                );
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(CandidateDecisionReference {
            application_id: state.0.ok_or_else(|| E::missing_field("application_id"))?,
            evidence_digest: state.1.ok_or_else(|| E::missing_field("evidence_digest"))?,
            kind: state.2.ok_or_else(|| E::missing_field("kind"))?,
            members: state.3.ok_or_else(|| E::missing_field("members"))?,
        })
    }
}

impl Node for EvidenceNode {
    type Value = CandidateDecisionReference;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum BindingFields {
    Id,
    Source,
    Candidate,
    Effective,
    Supersedes,
}

impl ObjectFields for BindingNode {
    type Field = BindingFields;
    type State = (
        Option<String>,
        Option<SourceObjectKey>,
        Option<AthleteCandidateId>,
        Option<EffectiveScope>,
        Option<Option<String>>,
    );
    const FIELD_COUNT: usize = 5;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            BindingFields::Id => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("id"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<TextNode>::new(meter))?);
            }
            BindingFields::Source => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("source"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<SourceNode>::new(meter))?);
            }
            BindingFields::Candidate => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("candidate"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<CandidateNode>::new(meter))?);
            }
            BindingFields::Effective => {
                if state.3.is_some() {
                    return Err(A::Error::duplicate_field("effective"));
                }
                state.3 = Some(map.next_value_seed(NodeSeed::<ScopeNode>::new(meter))?);
            }
            BindingFields::Supersedes => {
                if state.4.is_some() {
                    return Err(A::Error::duplicate_field("supersedes"));
                }
                state.4 = Some(map.next_value_seed(NodeSeed::<OptionalTextNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(BindingClaim {
            id: state.0.ok_or_else(|| E::missing_field("id"))?,
            source: state.1.ok_or_else(|| E::missing_field("source"))?,
            candidate: state.2.ok_or_else(|| E::missing_field("candidate"))?,
            effective: state.3.ok_or_else(|| E::missing_field("effective"))?,
            supersedes: state.4.ok_or_else(|| E::missing_field("supersedes"))?,
        })
    }
}

impl Node for BindingNode {
    type Value = BindingClaim;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum SourceFields {
    Namespace,
    Identifier,
}

impl ObjectFields for SourceNode {
    type Field = SourceFields;
    type State = (Option<SourceNamespace>, Option<String>);
    const FIELD_COUNT: usize = 2;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            SourceFields::Namespace => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("namespace"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<NamespaceNode>::new(meter))?);
            }
            SourceFields::Identifier => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("identifier"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<TextNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(SourceObjectKey {
            namespace: state.0.ok_or_else(|| E::missing_field("namespace"))?,
            identifier: state.1.ok_or_else(|| E::missing_field("identifier"))?,
        })
    }
}

impl Node for SourceNode {
    type Value = SourceObjectKey;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}
