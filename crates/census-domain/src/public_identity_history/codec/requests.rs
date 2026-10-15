use serde::de::{EnumAccess, Error, MapAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::model::ProfileDateRangeV1;

use super::super::{BindingClaim, CandidateDecisionReference, EffectiveScope, HistoryRequest};
use super::candidate_membership::MembershipBindNode;
use super::claims::{BindingNode, EvidenceNode};
use super::containers::{Bindings, ListNode, Meter, Node, NodeSeed};
use super::corrections::{RevocationNode, SplitNode};
use super::membership::{BindNode, MergeNode};
use super::object_decoder::{ObjectFields, ObjectSeed};

pub(super) struct RequestNode;
pub(super) struct ScopeNode;
struct EstablishNode;
struct MatchedNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum EstablishFields {
    Binding,
    Evidence,
}

impl ObjectFields for EstablishNode {
    type Field = EstablishFields;
    type State = (Option<BindingClaim>, Option<CandidateDecisionReference>);
    const FIELD_COUNT: usize = 2;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            EstablishFields::Binding => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("binding"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<BindingNode>::new(meter))?);
            }
            EstablishFields::Evidence => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(HistoryRequest::Establish {
            binding: state.0.ok_or_else(|| E::missing_field("binding"))?,
            evidence: state.1.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for EstablishNode {
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
enum MatchedFields {
    Bindings,
    Evidence,
}

impl ObjectFields for MatchedNode {
    type Field = MatchedFields;
    type State = (
        Option<Vec<BindingClaim>>,
        Option<CandidateDecisionReference>,
    );
    const FIELD_COUNT: usize = 2;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            MatchedFields::Bindings => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("bindings"));
                }
                state.0 = Some(
                    map.next_value_seed(NodeSeed::<ListNode<BindingNode, Bindings>>::new(meter))?,
                );
            }
            MatchedFields::Evidence => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("evidence"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<EvidenceNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(HistoryRequest::EstablishMatchedGroup {
            bindings: state.0.ok_or_else(|| E::missing_field("bindings"))?,
            evidence: state.1.ok_or_else(|| E::missing_field("evidence"))?,
        })
    }
}

impl Node for MatchedNode {
    type Value = HistoryRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
enum RequestVariant {
    Establish,
    EstablishMatchedGroup,
    Bind,
    BindCandidateMembership,
    Merge,
    Split,
    Revoke,
}

impl Node for RequestNode {
    type Value = HistoryRequest;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_enum(
            "HistoryRequest",
            &[
                "Establish",
                "EstablishMatchedGroup",
                "Bind",
                "BindCandidateMembership",
                "Merge",
                "Split",
                "Revoke",
            ],
            NodeSeed::<Self>::new(meter),
        )
    }
}

impl<'de> Visitor<'de> for NodeSeed<'_, RequestNode> {
    type Value = HistoryRequest;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one version-one history request")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (tag, variant) = data.variant::<RequestVariant>()?;
        match tag {
            RequestVariant::Establish => {
                variant.newtype_variant_seed(NodeSeed::<EstablishNode>::new(self.meter))
            }
            RequestVariant::EstablishMatchedGroup => {
                variant.newtype_variant_seed(NodeSeed::<MatchedNode>::new(self.meter))
            }
            RequestVariant::Bind => {
                variant.newtype_variant_seed(NodeSeed::<BindNode>::new(self.meter))
            }
            RequestVariant::BindCandidateMembership => {
                variant.newtype_variant_seed(NodeSeed::<MembershipBindNode>::new(self.meter))
            }
            RequestVariant::Merge => {
                variant.newtype_variant_seed(NodeSeed::<MergeNode>::new(self.meter))
            }
            RequestVariant::Split => variant
                .newtype_variant_seed(NodeSeed::<SplitNode>::new(self.meter))
                .map(HistoryRequest::Split),
            RequestVariant::Revoke => variant
                .newtype_variant_seed(NodeSeed::<RevocationNode>::new(self.meter))
                .map(HistoryRequest::Revoke),
        }
    }
}

#[derive(Deserialize)]
enum ScopeVariant {
    Unknown,
    Closed,
}

impl Node for ScopeNode {
    type Value = EffectiveScope;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_enum(
            "EffectiveScope",
            &["Unknown", "Closed"],
            NodeSeed::<Self>::new(meter),
        )
    }
}

impl<'de> Visitor<'de> for NodeSeed<'_, ScopeNode> {
    type Value = EffectiveScope;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an unknown or validated closed effective range")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (tag, variant) = data.variant::<ScopeVariant>()?;
        match tag {
            ScopeVariant::Unknown => variant.unit_variant().map(|()| EffectiveScope::Unknown),
            ScopeVariant::Closed => variant
                .newtype_variant::<ProfileDateRangeV1>()
                .map(EffectiveScope::Closed),
        }
    }
}
