use super::*;
use serde::{
    de::{SeqAccess, Visitor},
    Deserializer,
};
use std::fmt;

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
    #[serde(deserialize_with = "members")]
    members: Vec<AthleteCandidateId>,
    request: LocalIdentitySplitV1,
    observed_at: String,
    evidence_basis: LocalEvidenceBasisV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    sequence: u64,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Split {
    case_id: String,
    canonical: String,
    #[serde(deserialize_with = "members")]
    supersedes: Vec<SupersededMergeV1>,
    #[serde(deserialize_with = "members")]
    partitions: Vec<Partition>,
    #[serde(deserialize_with = "members")]
    unresolved_bindings: Vec<String>,
    #[serde(deserialize_with = "records")]
    affected_record_keys: Vec<String>,
    #[serde(deserialize_with = "claims")]
    dispositions: Vec<RetainedClaimDispositionV1>,
    rationale: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Partition {
    #[serde(deserialize_with = "members")]
    binding_ids: Vec<String>,
    representative: AthleteCandidateId,
}

impl<'de> Deserialize<'de> for LocalIdentitySplitV1 {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> Result<Self, D::Error> {
        let row = Split::deserialize(parser)?;
        Ok(Self {
            case_id: row.case_id,
            canonical: PublicAthleteId::parse(&row.canonical).map_err(serde::de::Error::custom)?,
            supersedes: row.supersedes,
            partitions: row
                .partitions
                .into_iter()
                .map(|partition| BindingPartition {
                    binding_ids: partition.binding_ids,
                    representative: partition.representative,
                })
                .collect(),
            unresolved_bindings: row.unresolved_bindings,
            affected_record_keys: row.affected_record_keys,
            dispositions: row.dispositions,
            rationale: row.rationale,
        })
    }
}

impl<'de> Deserialize<'de> for LocalIdentityAdjudicationRecordV1 {
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
            members: row.members,
            request: row.request,
            observed_at: row.observed_at,
            evidence_basis: row.evidence_basis,
        })
    }
}

fn members<'de, D: Deserializer<'de>, T: Deserialize<'de>>(parser: D) -> Result<Vec<T>, D::Error> {
    bounded::<D, T, 128>(parser)
}
fn records<'de, D: Deserializer<'de>, T: Deserialize<'de>>(parser: D) -> Result<Vec<T>, D::Error> {
    bounded::<D, T, 1024>(parser)
}
fn claims<'de, D: Deserializer<'de>, T: Deserialize<'de>>(parser: D) -> Result<Vec<T>, D::Error> {
    bounded::<D, T, 4096>(parser)
}
fn bounded<'de, D: Deserializer<'de>, T: Deserialize<'de>, const N: usize>(
    parser: D,
) -> Result<Vec<T>, D::Error> {
    struct Bounded<T, const N: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for Bounded<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "at most {N} manual adjudication entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            for _ in 0..=N {
                let Some(row) = input.next_element()? else {
                    return Ok(rows);
                };
                if rows.len() == N {
                    return Err(serde::de::Error::custom(
                        "manual adjudication collection bound",
                    ));
                }
                rows.try_reserve(1).map_err(serde::de::Error::custom)?;
                rows.push(row);
            }
            Err(serde::de::Error::custom(
                "manual adjudication collection did not end",
            ))
        }
    }
    parser.deserialize_seq(Bounded::<T, N>(std::marker::PhantomData))
}
