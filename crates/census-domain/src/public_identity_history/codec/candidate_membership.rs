use serde::de::{Error, MapAccess};
use serde::{Deserialize, Deserializer};

use super::super::{
    CandidateDecisionReference, CandidateMembershipClaimV1, HistoryRequest, PublicAthleteId,
    SourceObjectKey,
};
use super::atoms::{CandidateNode, PublicIdNode, TextNode};
use super::claims::{EvidenceNode, SourceNode};
use super::containers::{Meter, Node, NodeSeed};
use super::object_decoder::{ObjectFields, ObjectSeed};
use crate::model::AthleteCandidateId;

pub(super) struct MembershipNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum MembershipNodeFields {
    Id,
    Candidate,
    AnchorBindingId,
    Source,
}

impl ObjectFields for MembershipNode {
    type Field = MembershipNodeFields;
    type State = (
        Option<String>,
        Option<AthleteCandidateId>,
        Option<String>,
        Option<SourceObjectKey>,
    );
    const FIELD_COUNT: usize = 4;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            MembershipNodeFields::Id => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("id"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<TextNode>::new(meter))?);
            }
            MembershipNodeFields::Candidate => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("candidate"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<CandidateNode>::new(meter))?);
            }
            MembershipNodeFields::AnchorBindingId => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("anchor_binding_id"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<TextNode>::new(meter))?);
            }
            MembershipNodeFields::Source => {
                if state.3.is_some() {
                    return Err(A::Error::duplicate_field("source"));
                }
                state.3 = Some(map.next_value_seed(NodeSeed::<SourceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(CandidateMembershipClaimV1 {
            id: state.0.ok_or_else(|| E::missing_field("id"))?,
            candidate: state.1.ok_or_else(|| E::missing_field("candidate"))?,
            anchor_binding_id: state
                .2
                .ok_or_else(|| E::missing_field("anchor_binding_id"))?,
            source: state.3.ok_or_else(|| E::missing_field("source"))?,
        })
    }
}

impl Node for MembershipNode {
    type Value = CandidateMembershipClaimV1;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

pub(super) struct MembershipBindNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum MembershipBindNodeFields {
    Owner,
    Membership,
    Evidence,
}

impl ObjectFields for MembershipBindNode {
    type Field = MembershipBindNodeFields;
    type State = (
        Option<PublicAthleteId>,
        Option<CandidateMembershipClaimV1>,
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
            MembershipBindNodeFields::Owner => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("owner"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<PublicIdNode>::new(meter))?);
            }
            MembershipBindNodeFields::Membership => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("membership"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<MembershipNode>::new(meter))?);
            }
            MembershipBindNodeFields::Evidence => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(HistoryRequest::BindCandidateMembership {
            owner: state.0.ok_or_else(|| E::missing_field("owner"))?,
            membership: state.1.ok_or_else(|| E::missing_field("membership"))?,
            evidence: state.2.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for MembershipBindNode {
    type Value = HistoryRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}
