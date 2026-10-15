use std::fmt;

use crate::model::ProfileDateRangeV1;
use serde::de::{EnumAccess, Error, MapAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::super::{
    BindingClaim, CandidateDecisionReference, EffectiveScope, HistoryRequest, PublicAthleteId,
};
use super::atoms::PublicIdNode;
use super::containers::{Bindings, ListNode, Members, Meter, Node, NodeSeed};
use super::objects::{object_node, BindingNode, EvidenceNode, RevocationNode, SplitNode};

pub(super) struct RequestNode;
pub(super) struct ScopeNode;
struct EstablishNode;
struct MatchedNode;
struct BindNode;
struct MergeNode;

struct EstablishInput {
    binding: BindingClaim,
    evidence: CandidateDecisionReference,
}
struct MatchedInput {
    bindings: Vec<BindingClaim>,
    evidence: CandidateDecisionReference,
}
struct BindInput {
    owner: PublicAthleteId,
    binding: BindingClaim,
    evidence: CandidateDecisionReference,
}
struct MergeInput {
    canonical: PublicAthleteId,
    members: Vec<PublicAthleteId>,
    evidence: CandidateDecisionReference,
}

object_node!(EstablishNode, EstablishFields => EstablishInput {
    Binding => binding: BindingNode,
    Evidence => evidence: EvidenceNode,
});
object_node!(MatchedNode, MatchedFields => MatchedInput {
    Bindings => bindings: ListNode<BindingNode, Bindings>,
    Evidence => evidence: EvidenceNode,
});
object_node!(BindNode, BindFields => BindInput {
    Owner => owner: PublicIdNode,
    Binding => binding: BindingNode,
    Evidence => evidence: EvidenceNode,
});
object_node!(MergeNode, MergeFields => MergeInput {
    Canonical => canonical: PublicIdNode,
    Members => members: ListNode<PublicIdNode, Members>,
    Evidence => evidence: EvidenceNode,
});

#[derive(Deserialize)]
enum RequestVariant {
    Establish,
    EstablishMatchedGroup,
    Bind,
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

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one version-one history request")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (tag, variant) = data.variant::<RequestVariant>()?;
        match tag {
            RequestVariant::Establish => {
                let input =
                    variant.newtype_variant_seed(NodeSeed::<EstablishNode>::new(self.meter))?;
                Ok(HistoryRequest::Establish {
                    binding: input.binding,
                    evidence: input.evidence,
                })
            }
            RequestVariant::EstablishMatchedGroup => {
                let input =
                    variant.newtype_variant_seed(NodeSeed::<MatchedNode>::new(self.meter))?;
                Ok(HistoryRequest::EstablishMatchedGroup {
                    bindings: input.bindings,
                    evidence: input.evidence,
                })
            }
            RequestVariant::Bind => {
                let input = variant.newtype_variant_seed(NodeSeed::<BindNode>::new(self.meter))?;
                Ok(HistoryRequest::Bind {
                    owner: input.owner,
                    binding: input.binding,
                    evidence: input.evidence,
                })
            }
            RequestVariant::Merge => {
                let input = variant.newtype_variant_seed(NodeSeed::<MergeNode>::new(self.meter))?;
                Ok(HistoryRequest::Merge {
                    canonical: input.canonical,
                    members: input.members,
                    evidence: input.evidence,
                })
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

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
