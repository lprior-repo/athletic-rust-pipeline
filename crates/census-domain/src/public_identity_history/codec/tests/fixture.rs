use super::*;

pub(super) struct Fixture {
    pub(super) installation: StoreIdentity,
    pub(super) head: RegistryHead,
    pub(super) records: Vec<HistoryRecord>,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        let installation = StoreIdentity::parse(&"1".repeat(64))?;
        Ok(Self {
            head: genesis(&installation)?,
            installation,
            records: Vec::new(),
        })
    }

    pub(super) fn append(&mut self, request: HistoryRequest) -> TestResult<Vec<PublicAthleteId>> {
        let prepared = prepare_next(&self.installation, &self.head, request, ReplayLimits::SMALL)?;
        self.head = prepared.next_head;
        self.records.push(prepared.uncommitted_record);
        Ok(prepared.established_ids)
    }

    pub(super) fn one() -> TestResult<Self> {
        let mut fixture = Self::new()?;
        let _ids = fixture.append(establish("opaque candidate", "17")?)?;
        Ok(fixture)
    }
}

pub(super) fn candidate(value: &str) -> TestResult<AthleteCandidateId> {
    Ok(serde_json::from_str(&serde_json::to_string(value)?)?)
}

pub(super) fn evidence(
    id: &str,
    kind: CandidateEvidenceKind,
    members: &[&str],
) -> TestResult<CandidateDecisionReference> {
    Ok(CandidateDecisionReference {
        application_id: id.into(),
        evidence_digest: "a".repeat(64),
        kind,
        members: members
            .iter()
            .map(|member| candidate(member))
            .collect::<TestResult<_>>()?,
    })
}

pub(super) fn binding(id: &str, source: &str) -> TestResult<BindingClaim> {
    Ok(BindingClaim {
        id: format!("binding:{id}"),
        source: SourceObjectKey {
            namespace: SourceNamespace::TfrrsAthlete,
            identifier: source.into(),
        },
        candidate: candidate(id)?,
        effective: EffectiveScope::Unknown,
        supersedes: None,
    })
}

pub(super) fn establish(id: &str, source: &str) -> TestResult<HistoryRequest> {
    Ok(HistoryRequest::Establish {
        binding: binding(id, source)?,
        evidence: evidence(
            &format!("source:{id}"),
            CandidateEvidenceKind::SourceBound,
            &[id],
        )?,
    })
}

pub(super) fn complete() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    let first = fixture
        .append(HistoryRequest::EstablishMatchedGroup {
            bindings: vec![binding("a", "1")?, binding("b", "2")?],
            evidence: evidence("matched", CandidateEvidenceKind::SamePerson, &["a", "b"])?,
        })?
        .into_iter()
        .next()
        .ok_or("missing group ID")?;
    let _ids = fixture.append(HistoryRequest::Bind {
        owner: first.clone(),
        binding: binding("c", "3")?,
        evidence: evidence("bound", CandidateEvidenceKind::SamePerson, &["a", "c"])?,
    })?;
    let fourth = fixture
        .append(establish("d", "4")?)?
        .into_iter()
        .next()
        .ok_or("missing fourth ID")?;
    for (id, members) in [
        ("merge", vec![first.clone(), fourth]),
        ("redundant", vec![first.clone()]),
    ] {
        let _ids = fixture.append(HistoryRequest::Merge {
            canonical: first.clone(),
            members,
            evidence: evidence(id, CandidateEvidenceKind::SamePerson, &["a", "d"])?,
        })?;
    }
    let _ids = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: first.clone(),
        revoke_merges: vec![4],
        affected_record_keys: vec!["mark:1".into()],
        evidence: evidence(
            "revoke",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c", "d"],
        )?,
    }))?;
    let _ids = fixture.append(HistoryRequest::Split(SplitRequest {
        canonical: first,
        revoke_merges: vec![5],
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:a".into(), "binding:b".into(), "binding:c".into()],
                representative: candidate("a")?,
            },
            BindingPartition {
                binding_ids: vec!["binding:d".into()],
                representative: candidate("d")?,
            },
        ],
        unresolved_bindings: Vec::new(),
        affected_record_keys: vec!["mark:1".into()],
        evidence: evidence(
            "split",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c", "d"],
        )?,
    }))?;
    Ok(fixture)
}
