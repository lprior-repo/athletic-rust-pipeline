use super::*;

#[test]
fn new_membership_history_roundtrips_without_changing_earlier_records() -> TestResult {
    use serde::de::DeserializeSeed;
    let mut fixture = attached()?;
    let original = serde_json::to_vec(fixture.records.first().ok_or("missing source birth")?)?;
    let _births = fixture.append(withhold()?)?;
    let raw = serde_json::to_vec(&fixture.records)?;
    let mut parser = serde_json::Deserializer::from_slice(&raw);
    let decoded =
        CandidateHistorySeed::new(&fixture.installation, &fixture.head, ReplayLimits::SMALL)
            .deserialize(&mut parser)?;
    parser.end()?;
    check!(eq; decoded.records(), fixture.records.as_slice());
    check!(eq; serde_json::to_vec(decoded.records().first().ok_or("missing source birth")?)?, original);
    check!(eq; decoded.projection().resolve_candidate(&candidate("b"))?, PublicResolution::Unresolved);
    Ok(())
}

#[test]
fn membership_claim_text_and_graph_work_have_exact_boundaries() -> TestResult {
    let mut fixture = attached()?;
    let _births = fixture.append(withhold()?)?;
    let limits = ReplayLimits {
        graph_steps: 0,
        ..ReplayLimits::SMALL
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits).err(), Some(HistoryError::Limit("graph steps")));
    for (size, valid) in [(512, true), (513, false)] {
        let mut request = attachment("b")?;
        if let HistoryRequest::BindCandidateMembership { membership, .. } = &mut request {
            membership.id = "x".repeat(size);
        }
        let result = prepare_next(
            &fixture.installation,
            &fixture.head,
            request,
            ReplayLimits::SMALL,
        );
        check!(eq; result.is_ok(), valid);
    }
    Ok(())
}

#[test]
fn independent_source_split_retains_exact_membership_subgroup() -> TestResult {
    let mut fixture = attached()?;
    let _birth = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["b", "c"], "merge:bc")?;
    let split = SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![4],
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:a".into(), "membership:b".into()],
                representative: candidate("a"),
            },
            BindingPartition {
                binding_ids: vec!["binding:c".into()],
                representative: candidate("c"),
            },
        ],
        unresolved_bindings: vec![],
        affected_record_keys: vec!["all".into()],
        evidence: evidence(
            "split:source",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    };
    let ids = fixture.append(HistoryRequest::Split(split))?;
    check!(eq; ids.len(), 2);
    let projection = fixture.project()?;
    check!(eq; projection.resolve_candidate(&candidate("a"))?, projection.resolve_candidate(&candidate("b"))?);
    check!(
        projection.resolve_candidate(&candidate("a"))?
            != projection.resolve_candidate(&candidate("c"))?
    );
    Ok(())
}

#[test]
fn membership_edge_cannot_hide_detached_independent_source() -> TestResult {
    let mut fixture = attached()?;
    let _birth = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["b", "c"], "merge:bc")?;
    let _births = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![2],
        affected_record_keys: vec!["all".into()],
        evidence: evidence(
            "withhold:disconnected-source",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    }))?;
    check!(eq; fixture.project().err(), Some(HistoryError::SplitRequired));
    Ok(())
}

#[test]
fn membership_cannot_represent_a_split_successor() -> TestResult {
    let mut fixture = attached()?;
    let _birth = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["b", "c"], "merge:bc")?;
    let prior = fixture.project()?;
    let request = HistoryRequest::Split(SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![4],
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:a".into(), "membership:b".into()],
                representative: candidate("b"),
            },
            BindingPartition {
                binding_ids: vec!["binding:c".into()],
                representative: candidate("c"),
            },
        ],
        unresolved_bindings: vec![],
        affected_record_keys: vec!["all".into()],
        evidence: evidence(
            "split:membership-representative",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    });
    let prepared = prepare_next(
        &fixture.installation,
        &fixture.head,
        request,
        ReplayLimits::SMALL,
    )?;
    check!(eq; prior.clone().extend_candidate(&prepared.uncommitted_record).err(), Some(HistoryError::Partition));
    check!(eq; prior.head(), &fixture.head);
    check!(eq; prior.resolve_candidate(&candidate("a"))?, PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    Ok(())
}

#[test]
fn dateless_membership_cannot_choose_a_reused_provider_epoch() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut first = establish("a", "1");
    if let HistoryRequest::Establish { binding, .. } = &mut first {
        binding.effective = closed("2020-01-01", "2021-01-01")?;
    }
    let _birth = fixture.append(first)?;
    let mut next = establish("c", "1");
    if let HistoryRequest::Establish { binding, .. } = &mut next {
        binding.effective = closed("2022-01-01", "2023-01-01")?;
        binding.supersedes = Some("binding:a".into());
    }
    let _birth = fixture.append(next)?;
    let mut request = attachment("b")?;
    if let HistoryRequest::BindCandidateMembership {
        owner,
        membership,
        evidence: reference,
    } = &mut request
    {
        *owner = PublicAthleteId::parse(SECOND)?;
        membership.anchor_binding_id = "binding:c".into();
        *reference = evidence(
            "attach:reused",
            CandidateEvidenceKind::SamePerson,
            &["b", "c"],
        );
    }
    let prior = fixture.project()?;
    let prepared = prepare_next(
        &fixture.installation,
        &fixture.head,
        request,
        ReplayLimits::SMALL,
    )?;
    check!(eq; prior.clone().extend_candidate(&prepared.uncommitted_record).err(), Some(HistoryError::SourceEpochEvidenceRequired));
    check!(eq; prior.resolve_candidate(&candidate("b"))?, PublicResolution::Unknown);
    check!(eq; prior.head(), &fixture.head);
    Ok(())
}

#[test]
fn membership_preserves_all_members_of_genuine_reviewed_edge() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _birth = fixture.append(establish("a", "1"))?;
    let _births = fixture.append(HistoryRequest::Bind {
        owner: PublicAthleteId::parse(FIRST)?,
        binding: binding("c", "3"),
        evidence: evidence("bind:ac", CandidateEvidenceKind::SamePerson, &["a", "c"]),
    })?;
    let mut request = attachment("b")?;
    if let HistoryRequest::BindCandidateMembership {
        evidence: reference,
        ..
    } = &mut request
    {
        *reference = evidence(
            "attach:abc",
            CandidateEvidenceKind::SamePerson,
            &["a", "b", "c"],
        );
    }
    let _births = fixture.append(request)?;
    let _births = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![2],
        affected_record_keys: vec!["all".into()],
        evidence: evidence(
            "revoke:ac",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    }))?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_candidate(&candidate("c"))?, PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(eq; projection.withheld_memberships(4), Some([].as_slice()));
    check!(eq; projection.preview_withheld_memberships(&PublicAthleteId::parse(FIRST)?, 3).err(), Some(HistoryError::SplitRequired));
    Ok(())
}

#[test]
fn split_then_withhold_preserves_independent_representative() -> TestResult {
    let mut fixture = attached()?;
    let _birth = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["b", "c"], "merge:bc")?;
    let ids = fixture.append(HistoryRequest::Split(SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![4],
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:a".into(), "membership:b".into()],
                representative: candidate("a"),
            },
            BindingPartition {
                binding_ids: vec!["binding:c".into()],
                representative: candidate("c"),
            },
        ],
        unresolved_bindings: vec![],
        affected_record_keys: vec!["all".into()],
        evidence: evidence(
            "split:ac",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    }))?;
    let canonical = ids.first().ok_or("first split successor")?.clone();
    let before = fixture.project()?;
    check!(eq; before.preview_withheld_memberships(&canonical, 2)?, vec!["membership:b".to_owned()]);
    let _births = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: canonical.clone(),
        revoke_merges: vec![2],
        affected_record_keys: vec!["all".into()],
        evidence: evidence(
            "withhold:split-b",
            CandidateEvidenceKind::Correction,
            &["a", "b"],
        ),
    }))?;
    let after = fixture.project()?;
    check!(eq; after.resolve_candidate(&candidate("a"))?, before.resolve_candidate(&candidate("a"))?);
    check!(eq; after.resolve_candidate(&candidate("b"))?, PublicResolution::Unresolved);
    check!(
        matches!(after.resolve_public(&canonical)?, PublicResolution::Canonical(row) if row.representative_candidate == candidate("a"))
    );
    Ok(())
}
