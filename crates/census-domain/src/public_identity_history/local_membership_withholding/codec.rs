use super::super::local_adjudication::codec::{claims, members, records};
use super::*;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    case_id: String,
    canonical: String,
    attachment: MembershipAttachmentV1,
    #[serde(deserialize_with = "members")]
    expected_withheld_memberships: Vec<String>,
    #[serde(deserialize_with = "records")]
    affected_record_keys: Vec<String>,
    rationale: String,
}
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
    authority_context: String,
    affected_context: String,
    #[serde(deserialize_with = "members")]
    members: Vec<AthleteCandidateId>,
    request: LocalMembershipWithholdV1,
    observed_at: String,
    #[serde(deserialize_with = "claims")]
    dispositions: Vec<RetainedClaimDispositionV1>,
}
impl<'de> Deserialize<'de> for LocalMembershipWithholdV1 {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> Result<Self, D::Error> {
        let row = Request::deserialize(parser)?;
        Ok(Self {
            case_id: row.case_id,
            canonical: PublicAthleteId::parse(&row.canonical).map_err(serde::de::Error::custom)?,
            attachment: row.attachment,
            expected_withheld_memberships: row.expected_withheld_memberships,
            affected_record_keys: row.affected_record_keys,
            rationale: row.rationale,
        })
    }
}
impl<'de> Deserialize<'de> for LocalMembershipWithholdingRecordV1 {
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
            authority_context: row.authority_context,
            affected_context: row.affected_context,
            members: row.members,
            request: row.request,
            observed_at: row.observed_at,
            dispositions: row.dispositions,
        })
    }
}
