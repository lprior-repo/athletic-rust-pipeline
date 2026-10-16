use super::*;

fn members(request: &mut HistoryRequest) -> &mut Vec<AthleteCandidateId> {
    match request {
        HistoryRequest::Establish { evidence, .. }
        | HistoryRequest::EstablishMatchedGroup { evidence, .. }
        | HistoryRequest::Bind { evidence, .. }
        | HistoryRequest::BindCandidateMembership { evidence, .. }
        | HistoryRequest::Merge { evidence, .. } => &mut evidence.members,
        HistoryRequest::Split(split) => &mut split.evidence.members,
        HistoryRequest::Revoke(revocation) => &mut revocation.evidence.members,
    }
}

fn variants() -> TestResult<Vec<HistoryRequest>> {
    let proof = evidence("match:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]);
    Ok(vec![
        HistoryRequest::Establish {
            binding: binding("a", "1"),
            evidence: proof.clone(),
        },
        HistoryRequest::EstablishMatchedGroup {
            bindings: vec![binding("a", "1"), binding("b", "2")],
            evidence: proof.clone(),
        },
        HistoryRequest::Bind {
            owner: PublicAthleteId::parse(FIRST)?,
            binding: binding("b", "2"),
            evidence: proof.clone(),
        },
        HistoryRequest::BindCandidateMembership {
            owner: PublicAthleteId::parse(FIRST)?,
            membership: CandidateMembershipClaimV1 {
                id: "membership:b".into(),
                candidate: candidate("b"),
                anchor_binding_id: "binding:a".into(),
                source: binding("b", "2").source,
            },
            evidence: proof.clone(),
        },
        HistoryRequest::Merge {
            canonical: PublicAthleteId::parse(FIRST)?,
            members: vec![
                PublicAthleteId::parse(FIRST)?,
                PublicAthleteId::parse(SECOND)?,
            ],
            evidence: proof.clone(),
        },
        HistoryRequest::Split(split_request(&[&["a", "b"], &["c"]], vec![4, 5])?),
        HistoryRequest::Revoke(RevocationRequest {
            canonical: PublicAthleteId::parse(FIRST)?,
            revoke_merges: vec![4, 5],
            affected_record_keys: vec!["performance:1".into(), "performance:2".into()],
            evidence: proof,
        }),
    ])
}

#[test]
fn evidence_order_and_duplicate_refusals_cover_every_request_variant() -> TestResult {
    for request in variants()? {
        let normalized = request_order::normalize(request)?;
        let mut descending = normalized.clone();
        members(&mut descending).swap(0, 1);
        check!(eq; request_order::require_canonical(&descending),
            Err(HistoryError::Invalid("noncanonical request order")));
        let mut duplicate = normalized;
        let items = members(&mut duplicate);
        items[1] = items[0].clone();
        check!(eq; request_order::require_canonical(&duplicate),
            Err(HistoryError::Duplicate("evidence members")));
    }
    Ok(())
}
