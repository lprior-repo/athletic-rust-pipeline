use std::fmt;

use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::super::{
    BindingClaim, BindingPartition, CandidateDecisionReference, HistoryRecord, RegistryHead,
    RevocationRequest, SourceObjectKey, SplitRequest,
};
use super::atoms::{
    CandidateNode, DigestNode, EvidenceDigestNode, EvidenceKindNode, InstallationNode, NumberNode,
    OptionalTextNode, PublicIdNode, TextNode, VersionNode,
};
use super::containers::{Affected, ListNode, Members, Meter, Node, NodeSeed, RecordNode};
use super::namespace::NamespaceNode;
use super::requests::{RequestNode, ScopeNode};

macro_rules! object_node {
    ($node:ident, $fields:ident => $($target:ident)::+ {
        $($variant:ident => $field:ident : $child:ty),+ $(,)?
    }) => {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum $fields { $($variant),+ }

        impl Node for $node {
            type Value = $($target)::+;

            fn decode<'de, D: Deserializer<'de>>(
                meter: &mut Meter,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_map(NodeSeed::<Self>::new(meter))
            }
        }

        impl<'de> Visitor<'de> for NodeSeed<'_, $node> {
            type Value = $($target)::+;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a complete history object without unknown or duplicate fields")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                $(let mut $field = None;)+
                while let Some(field) = map.next_key::<$fields>()? {
                    match field {
                        $($fields::$variant => {
                            if $field.is_some() {
                                return Err(A::Error::duplicate_field(stringify!($field)));
                            }
                            $field = Some(map.next_value_seed(NodeSeed::<$child>::new(self.meter))?);
                        })+
                    }
                }
                Ok($($target)::+ {
                    $($field: $field.ok_or_else(|| A::Error::missing_field(stringify!($field)))?),+
                })
            }
        }
    };
}

pub(super) use object_node;

pub(super) struct HeadNode;
pub(super) struct EvidenceNode;
pub(super) struct BindingNode;
pub(super) struct SourceNode;
pub(super) struct PartitionNode;
pub(super) struct SplitNode;
pub(super) struct RevocationNode;

object_node!(RecordNode, RecordFields => HistoryRecord {
    Version => version: VersionNode,
    Installation => installation: InstallationNode,
    Sequence => sequence: NumberNode,
    Previous => previous: HeadNode,
    Request => request: RequestNode,
    Digest => digest: DigestNode,
});

object_node!(HeadNode, HeadFields => RegistryHead {
    Sequence => sequence: NumberNode,
    Digest => digest: DigestNode,
});

object_node!(EvidenceNode, EvidenceFields => CandidateDecisionReference {
    ApplicationId => application_id: TextNode,
    EvidenceDigest => evidence_digest: EvidenceDigestNode,
    Kind => kind: EvidenceKindNode,
    Members => members: ListNode<CandidateNode, Members>,
});

object_node!(BindingNode, BindingFields => BindingClaim {
    Id => id: TextNode,
    Source => source: SourceNode,
    Candidate => candidate: CandidateNode,
    Effective => effective: ScopeNode,
    Supersedes => supersedes: OptionalTextNode,
});

object_node!(SourceNode, SourceFields => SourceObjectKey {
    Namespace => namespace: NamespaceNode,
    Identifier => identifier: TextNode,
});

object_node!(PartitionNode, PartitionFields => BindingPartition {
    BindingIds => binding_ids: ListNode<TextNode, Members>,
    Representative => representative: CandidateNode,
});

object_node!(SplitNode, SplitFields => SplitRequest {
    Canonical => canonical: PublicIdNode,
    RevokeMerges => revoke_merges: ListNode<NumberNode, Members>,
    Partitions => partitions: ListNode<PartitionNode, Members>,
    UnresolvedBindings => unresolved_bindings: ListNode<TextNode, Members>,
    AffectedRecordKeys => affected_record_keys: ListNode<TextNode, Affected>,
    Evidence => evidence: EvidenceNode,
});

object_node!(RevocationNode, RevocationFields => RevocationRequest {
    Canonical => canonical: PublicIdNode,
    RevokeMerges => revoke_merges: ListNode<NumberNode, Members>,
    AffectedRecordKeys => affected_record_keys: ListNode<TextNode, Affected>,
    Evidence => evidence: EvidenceNode,
});
