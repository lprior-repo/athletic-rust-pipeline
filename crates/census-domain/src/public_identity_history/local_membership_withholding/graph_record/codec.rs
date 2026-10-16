use super::*;
use crate::public_identity_history::local_adjudication::codec::{claims, members};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    sequence: u64,
    digest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    policy: String,
    store: StoreIdentity,
    operation: String,
    expected_head: Head,
    evidence_generation: u64,
    derived_generation: u64,
    snapshot_sequence: u64,
    graph: crate::public_identity_history::GraphAuthorityReferenceV1,
    #[serde(deserialize_with = "members")]
    members: Vec<AthleteCandidateId>,
    request: LocalMembershipWithholdV1,
    observed_at: String,
    #[serde(deserialize_with = "claims")]
    dispositions: Vec<RetainedClaimDispositionV1>,
}
impl<'de> Deserialize<'de> for LocalMembershipWithholdingRecordV2 {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> Result<Self, D::Error> {
        let row = Record::deserialize(parser)?;
        Ok(Self {
            version: row.version,
            policy: row.policy,
            store: row.store,
            operation: row.operation,
            expected_head: RegistryHead {
                sequence: row.expected_head.sequence,
                digest: row.expected_head.digest,
            },
            evidence_generation: row.evidence_generation,
            derived_generation: row.derived_generation,
            snapshot_sequence: row.snapshot_sequence,
            graph: row.graph,
            members: row.members,
            request: row.request,
            observed_at: row.observed_at,
            dispositions: row.dispositions,
        })
    }
}
