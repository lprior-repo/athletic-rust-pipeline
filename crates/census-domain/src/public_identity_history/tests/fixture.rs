use crate::model::{
    AthleteCandidateId, ProfileDate, ProfileDateRangeV1, SourceNamespace, StoreIdentity,
};

use super::*;

pub(in super::super) const FIRST: &str =
    "ath_public_v1_2e4fb039913a48c1fc1c41f2308f4ffc47f0e2b356e2cf14887893066269fa60";
pub(in super::super) const SECOND: &str =
    "ath_public_v1_afcea59d6afd88c8f0174ae3261f8fcd425bef80224ccd8a1a9a12981aa2c339";
pub(in super::super) const THIRD: &str =
    "ath_public_v1_6522bfe506b5c67435c61ed3da84dd92e1e65f8889a239cc56b3c0b6c1e20266";
pub(in super::super) const SPLIT_A: &str =
    "ath_public_v1_44c9ffde4a8068cb6d975d01cf404f8c76017792dab789899b4a63de6699280c";
pub(in super::super) const SPLIT_B: &str =
    "ath_public_v1_3eca8b5f6df95cfface1b198cef6e6f374b777159a5336f3b4e057a375a71c7d";
pub(in super::super) const SPLIT_C: &str =
    "ath_public_v1_ce459e4b42ad30398400daac53f9c3f7ea40ad177191f57f9da268e42ce490a9";

pub(in super::super) fn candidate(label: &str) -> AthleteCandidateId {
    AthleteCandidateId::mint("ath", &[label])
}

pub(in super::super) fn evidence(
    id: &str,
    kind: CandidateEvidenceKind,
    members: &[&str],
) -> CandidateDecisionReference {
    CandidateDecisionReference {
        application_id: id.into(),
        evidence_digest: "a".repeat(64),
        kind,
        members: members.iter().map(|label| candidate(label)).collect(),
    }
}

pub(in super::super) fn binding(label: &str, source_id: &str) -> BindingClaim {
    BindingClaim {
        id: format!("binding:{label}"),
        source: SourceObjectKey {
            namespace: SourceNamespace::TfrrsAthlete,
            identifier: source_id.into(),
        },
        candidate: candidate(label),
        effective: EffectiveScope::Unknown,
        supersedes: None,
    }
}

pub(in super::super) fn closed(from: &str, through: &str) -> TestResult<EffectiveScope> {
    Ok(EffectiveScope::Closed(ProfileDateRangeV1::new(
        ProfileDate::parse(from)?,
        ProfileDate::parse(through)?,
    )?))
}

pub(in super::super) fn establish(label: &str, source_id: &str) -> HistoryRequest {
    HistoryRequest::Establish {
        binding: binding(label, source_id),
        evidence: evidence(
            &format!("source:{label}"),
            CandidateEvidenceKind::SourceBound,
            &[label],
        ),
    }
}

pub(in super::super) fn successor(
    id: &str,
    label: &str,
    sequence: u64,
    position: u32,
) -> TestResult<ResolvedSuccessor> {
    Ok(ResolvedSuccessor {
        public_id: PublicAthleteId::parse(id)?,
        representative_candidate: candidate(label),
        established: EstablishmentOrdinal { sequence, position },
    })
}

pub(in super::super) struct Fixture {
    pub installation: StoreIdentity,
    pub head: RegistryHead,
    pub records: Vec<HistoryRecord>,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let installation = StoreIdentity::parse(&"1".repeat(64))?;
        Ok(Self {
            head: genesis(&installation)?,
            installation,
            records: Vec::new(),
        })
    }

    pub fn append(&mut self, request: HistoryRequest) -> TestResult<Vec<PublicAthleteId>> {
        let prepared = prepare_next(&self.installation, &self.head, request, ReplayLimits::SMALL)?;
        self.head = prepared.next_head;
        self.records.push(prepared.uncommitted_record);
        Ok(prepared.established_ids)
    }

    pub fn project(&self) -> Result<HistoryProjection, HistoryError> {
        replay_candidate_history(
            &self.installation,
            &self.head,
            &self.records,
            ReplayLimits::SMALL,
        )
    }

    pub fn merge(
        &mut self,
        canonical: &str,
        members: &[&str],
        candidates: &[&str],
        application: &str,
    ) -> TestResult {
        let _births = self.append(HistoryRequest::Merge {
            canonical: PublicAthleteId::parse(canonical)?,
            members: members
                .iter()
                .map(|id| PublicAthleteId::parse(id))
                .collect::<Result<_, _>>()?,
            evidence: evidence(application, CandidateEvidenceKind::SamePerson, candidates),
        })?;
        Ok(())
    }
}

pub(in super::super) fn chain() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    for (label, source) in [("a", "1"), ("b", "2"), ("c", "3")] {
        let _births = fixture.append(establish(label, source))?;
    }
    fixture.merge(FIRST, &[FIRST, SECOND], &["a", "b"], "merge:ab")?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["b", "c"], "merge:bc")?;
    Ok(fixture)
}

pub(in super::super) fn split_request(
    groups: &[&[&str]],
    revoked: Vec<u64>,
) -> TestResult<SplitRequest> {
    let partitions = groups
        .iter()
        .map(|labels| {
            let representative = labels.first().ok_or("empty fixture partition")?;
            Ok(BindingPartition {
                binding_ids: labels
                    .iter()
                    .map(|label| format!("binding:{label}"))
                    .collect(),
                representative: candidate(representative),
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: revoked,
        partitions,
        unresolved_bindings: Vec::new(),
        affected_record_keys: vec!["performance:1".into(), "performance:2".into()],
        evidence: evidence(
            "correction:1",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    })
}
