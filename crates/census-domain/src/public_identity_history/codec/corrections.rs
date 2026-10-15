use serde::de::{Error, MapAccess};
use serde::{Deserialize, Deserializer};

use crate::model::AthleteCandidateId;

use super::super::{
    BindingPartition, CandidateDecisionReference, PublicAthleteId, RevocationRequest, SplitRequest,
};
use super::atoms::{CandidateNode, NumberNode, PublicIdNode, TextNode};
use super::claims::EvidenceNode;
use super::containers::{Affected, ListNode, Members, Meter, Node, NodeSeed};
use super::object_decoder::{ObjectFields, ObjectSeed};

pub(super) struct PartitionNode;
pub(super) struct SplitNode;
pub(super) struct RevocationNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum PartitionFields {
    BindingIds,
    Representative,
}

impl ObjectFields for PartitionNode {
    type Field = PartitionFields;
    type State = (Option<Vec<String>>, Option<AthleteCandidateId>);
    const FIELD_COUNT: usize = 2;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            PartitionFields::BindingIds => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("binding_ids"));
                }
                state.0 =
                    Some(map.next_value_seed(NodeSeed::<ListNode<TextNode, Members>>::new(meter))?);
            }
            PartitionFields::Representative => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("representative"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<CandidateNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(BindingPartition {
            binding_ids: state.0.ok_or_else(|| E::missing_field("binding_ids"))?,
            representative: state.1.ok_or_else(|| E::missing_field("representative"))?,
        })
    }
}

impl Node for PartitionNode {
    type Value = BindingPartition;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum SplitFields {
    Canonical,
    RevokeMerges,
    Partitions,
    UnresolvedBindings,
    AffectedRecordKeys,
    Evidence,
}

impl ObjectFields for SplitNode {
    type Field = SplitFields;
    type State = (
        Option<PublicAthleteId>,
        Option<Vec<u64>>,
        Option<Vec<BindingPartition>>,
        Option<Vec<String>>,
        Option<Vec<String>>,
        Option<CandidateDecisionReference>,
    );
    const FIELD_COUNT: usize = 6;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            SplitFields::Canonical => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("canonical"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<PublicIdNode>::new(meter))?);
            }
            SplitFields::RevokeMerges => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("revoke_merges"));
                }
                state.1 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<NumberNode, Members>>::new(meter))?,
                );
            }
            SplitFields::Partitions => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("partitions"));
                }
                state.2 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<PartitionNode, Members>>::new(meter))?,
                );
            }
            SplitFields::UnresolvedBindings => {
                if state.3.is_some() {
                    return Err(A::Error::duplicate_field("unresolved_bindings"));
                }
                state.3 =
                    Some(map.next_value_seed(NodeSeed::<ListNode<TextNode, Members>>::new(meter))?);
            }
            SplitFields::AffectedRecordKeys => {
                if state.4.is_some() {
                    return Err(A::Error::duplicate_field("affected_record_keys"));
                }
                state.4 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<TextNode, Affected>>::new(meter))?,
                );
            }
            SplitFields::Evidence => {
                if state.5.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.5 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(SplitRequest {
            canonical: state.0.ok_or_else(|| E::missing_field("canonical"))?,
            revoke_merges: state.1.ok_or_else(|| E::missing_field("revoke_merges"))?,
            partitions: state.2.ok_or_else(|| E::missing_field("partitions"))?,
            unresolved_bindings: state
                .3
                .ok_or_else(|| E::missing_field("unresolved_bindings"))?,
            affected_record_keys: state
                .4
                .ok_or_else(|| E::missing_field("affected_record_keys"))?,
            evidence: state.5.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for SplitNode {
    type Value = SplitRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum RevocationFields {
    Canonical,
    RevokeMerges,
    AffectedRecordKeys,
    Evidence,
}

impl ObjectFields for RevocationNode {
    type Field = RevocationFields;
    type State = (
        Option<PublicAthleteId>,
        Option<Vec<u64>>,
        Option<Vec<String>>,
        Option<CandidateDecisionReference>,
    );
    const FIELD_COUNT: usize = 4;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            RevocationFields::Canonical => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("canonical"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<PublicIdNode>::new(meter))?);
            }
            RevocationFields::RevokeMerges => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("revoke_merges"));
                }
                state.1 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<NumberNode, Members>>::new(meter))?,
                );
            }
            RevocationFields::AffectedRecordKeys => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("affected_record_keys"));
                }
                state.2 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<TextNode, Affected>>::new(meter))?,
                );
            }
            RevocationFields::Evidence => {
                if state.3.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.3 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(RevocationRequest {
            canonical: state.0.ok_or_else(|| E::missing_field("canonical"))?,
            revoke_merges: state.1.ok_or_else(|| E::missing_field("revoke_merges"))?,
            affected_record_keys: state
                .2
                .ok_or_else(|| E::missing_field("affected_record_keys"))?,
            evidence: state.3.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for RevocationNode {
    type Value = RevocationRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}
