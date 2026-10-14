use super::*;

const THIRD_B: &str =
    "ath_public_v1_e6fe93d36b5061cf1b6af94ea93543c2ec17bf1636edafddcb4f6d6785231f73";

fn split_group(revoked: Vec<u64>) -> TestResult<SplitRequest> {
    Ok(SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: revoked,
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:a".into()],
                representative: candidate("a"),
            },
            BindingPartition {
                binding_ids: vec!["binding:z".into()],
                representative: candidate("z"),
            },
        ],
        unresolved_bindings: Vec::new(),
        affected_record_keys: vec!["performance:1".into()],
        evidence: evidence(
            "correction:az",
            CandidateEvidenceKind::Correction,
            &["a", "z"],
        ),
    })
}

#[test]
fn splitting_the_initial_group_preserves_birth_bytes_and_unrelated_id() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(group())?;
    let _second = fixture.append(establish("b", "3"))?;
    let initial = serde_json::to_vec(fixture.records.first().ok_or("missing birth")?)?;
    check!(eq; fixture.append(HistoryRequest::Split(split_group(vec![1])?))?,
        vec![PublicAthleteId::parse(THIRD)?, PublicAthleteId::parse(THIRD_B)?]);
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?,
    PublicResolution::Ambiguous {
        successors: vec![successor(THIRD, "a", 3, 0)?, successor(THIRD_B, "z", 3, 1)?],
        unresolved_bindings: false,
        changed_at: 3,
        revoked_merges: vec![1],
    });
    check!(eq; projection.resolve_candidate(&candidate("a"))?,
        PublicResolution::Canonical(successor(THIRD, "a", 3, 0)?));
    check!(eq; projection.resolve_candidate(&candidate("z"))?,
        PublicResolution::Canonical(successor(THIRD_B, "z", 3, 1)?));
    check!(eq; projection.resolve_candidate(&candidate("b"))?,
        PublicResolution::Canonical(successor(SECOND, "b", 2, 0)?));
    check!(eq; projection.revoked_merges().collect::<Vec<_>>(), vec![(1, 3)]);
    check!(eq; serde_json::to_vec(fixture.records.first().ok_or("missing birth")?)?, initial);
    Ok(())
}

#[test]
fn the_initial_connection_cannot_be_cut_by_revoking_only_a_later_edge() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    fixture.merge(FIRST, &[FIRST], &["a", "z"], "matched:independent")?;
    let _split = fixture.append(HistoryRequest::Split(split_group(vec![2])?))?;
    check!(eq; fixture.project().err(), Some(HistoryError::SurvivingMerge));
    Ok(())
}

#[test]
fn same_person_evidence_cannot_authorize_initial_group_split() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    let mut split = split_group(vec![1])?;
    split.evidence.kind = CandidateEvidenceKind::SamePerson;
    let _split = fixture.append(HistoryRequest::Split(split))?;
    check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    Ok(())
}

#[test]
fn removing_the_only_group_edge_requires_explicit_split_successors() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    let _revoked = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![1],
        affected_record_keys: vec!["performance:1".into()],
        evidence: evidence(
            "correction:az",
            CandidateEvidenceKind::Correction,
            &["a", "z"],
        ),
    }))?;
    check!(eq; fixture.project().err(), Some(HistoryError::SplitRequired));
    Ok(())
}

#[test]
fn exact_group_edge_revocation_retains_independent_connectivity_and_id() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    fixture.merge(FIRST, &[FIRST], &["a", "z"], "matched:independent")?;
    check!(eq; fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![1],
        affected_record_keys: vec!["performance:1".into()],
        evidence: evidence("correction:az", CandidateEvidenceKind::Correction, &["a", "z"]),
    }))?, Vec::<PublicAthleteId>::new());
    let projection = fixture.project()?;
    check!(eq; projection.resolve_candidate(&candidate("z"))?,
        PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(eq; projection.correction_history(&PublicAthleteId::parse(FIRST)?), Some(&[1][..]));
    check!(eq; projection.revoked_merges().collect::<Vec<_>>(), vec![(1, 3)]);
    check!(eq; projection.public_ids().count(), 1);
    Ok(())
}

#[test]
fn splitting_initial_evidence_preserves_a_later_unaffected_connection() -> TestResult {
    const FOURTH_A: &str =
        "ath_public_v1_c0dc942c0bf86d0b7d67f40ac8274fda6ded228b911dc9b2824e83decabc6978";
    const FOURTH_B: &str =
        "ath_public_v1_8299d9c6e1ed81b6b49ff9c5966bccaa0442954ad0817ce3a93f566ac4fd1566";
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(group())?;
    let _second = fixture.append(establish("b", "3"))?;
    fixture.merge(FIRST, &[FIRST, SECOND], &["z", "b"], "matched:zb")?;
    let historical = serde_json::to_vec(&fixture.records)?;
    let _split = fixture.append(HistoryRequest::Split(SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![1],
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:a".into()],
                representative: candidate("a"),
            },
            BindingPartition {
                binding_ids: vec!["binding:z".into(), "binding:b".into()],
                representative: candidate("b"),
            },
        ],
        unresolved_bindings: Vec::new(),
        affected_record_keys: vec!["performance:1".into(), "performance:2".into()],
        evidence: evidence(
            "correction:azb",
            CandidateEvidenceKind::Correction,
            &["a", "z", "b"],
        ),
    }))?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?,
    PublicResolution::Ambiguous {
        successors: vec![successor(FOURTH_B, "b", 4, 1)?, successor(FOURTH_A, "a", 4, 0)?],
        unresolved_bindings: false,
        changed_at: 4,
        revoked_merges: vec![1],
    });
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?,
    PublicResolution::RevokedRedirect {
        successor: successor(FOURTH_B, "b", 4, 1)?,
        changed_at: 4,
        revoked_merges: vec![1],
    });
    for label in ["z", "b"] {
        check!(eq; projection.resolve_candidate(&candidate(label))?,
            PublicResolution::Canonical(successor(FOURTH_B, "b", 4, 1)?));
    }
    check!(eq; projection.revoked_merges().collect::<Vec<_>>(), vec![(1, 4)]);
    check!(eq; serde_json::to_vec(fixture.records.get(..3).ok_or("missing prior history")?)?, historical);
    Ok(())
}
