use serde::de::{Error, MapAccess};
use serde::{Deserialize, Deserializer};

use super::super::{BindingClaim, CandidateDecisionReference, HistoryRequest, PublicAthleteId};
use super::atoms::PublicIdNode;
use super::claims::{BindingNode, EvidenceNode};
use super::containers::{ListNode, Members, Meter, Node, NodeSeed};
use super::object_decoder::{ObjectFields, ObjectSeed};

pub(super) struct BindNode;
pub(super) struct MergeNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum BindFields {
    Owner,
    Binding,
    Evidence,
}

impl ObjectFields for BindNode {
    type Field = BindFields;
    type State = (
        Option<PublicAthleteId>,
        Option<BindingClaim>,
        Option<CandidateDecisionReference>,
    );
    const FIELD_COUNT: usize = 3;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            BindFields::Owner => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("owner"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<PublicIdNode>::new(meter))?);
            }
            BindFields::Binding => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("binding"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<BindingNode>::new(meter))?);
            }
            BindFields::Evidence => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(HistoryRequest::Bind {
            owner: state.0.ok_or_else(|| E::missing_field("owner"))?,
            binding: state.1.ok_or_else(|| E::missing_field("binding"))?,
            evidence: state.2.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for BindNode {
    type Value = HistoryRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum MergeFields {
    Canonical,
    Members,
    Evidence,
}

impl ObjectFields for MergeNode {
    type Field = MergeFields;
    type State = (
        Option<PublicAthleteId>,
        Option<Vec<PublicAthleteId>>,
        Option<CandidateDecisionReference>,
    );
    const FIELD_COUNT: usize = 3;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            MergeFields::Canonical => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("canonical"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<PublicIdNode>::new(meter))?);
            }
            MergeFields::Members => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("members"));
                }
                state.1 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<PublicIdNode, Members>>::new(meter))?,
                );
            }
            MergeFields::Evidence => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(HistoryRequest::Merge {
            canonical: state.0.ok_or_else(|| E::missing_field("canonical"))?,
            members: state.1.ok_or_else(|| E::missing_field("members"))?,
            evidence: state.2.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for MergeNode {
    type Value = HistoryRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}
