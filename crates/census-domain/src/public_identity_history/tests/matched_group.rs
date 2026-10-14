use super::fixture::*;
use super::*;

mod accepted;
mod corrections;

fn group() -> HistoryRequest {
    HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("z", "2"), binding("a", "1")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["z", "a"]),
    }
}

#[test]
fn one_matched_event_establishes_one_literal_id_and_every_binding() -> TestResult {
    let mut fixture = Fixture::new()?;
    check!(eq; fixture.append(group())?, vec![PublicAthleteId::parse(FIRST)?]);
    check!(eq; fixture.records.len(), 1);
    check!(eq; fixture.head.sequence, 1);
    let projection = fixture.project()?;
    check!(eq; projection.public_ids().count(), 1);
    for label in ["a", "z"] {
        check!(eq; projection.resolve_candidate(&candidate(label))?,
            PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
        check!(eq; projection.resolve_binding(&format!("binding:{label}"))?,
        BindingResolution::Resolved {
            identity: PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?),
            established_at: 1,
            superseded_by: None,
        });
    }
    check!(eq; projection.binding_claim("binding:a"), Some(&binding("a", "1")));
    check!(eq; projection.binding_claim("binding:z"), Some(&binding("z", "2")));
    Ok(())
}

#[test]
fn reversing_bindings_and_members_preserves_the_exact_event_and_birth_id() -> TestResult {
    let fixture = Fixture::new()?;
    let reversed = HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), binding("z", "2")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    };
    let first = prepare_next(
        &fixture.installation,
        &fixture.head,
        group(),
        ReplayLimits::SMALL,
    )?;
    let second = prepare_next(
        &fixture.installation,
        &fixture.head,
        reversed,
        ReplayLimits::SMALL,
    )?;
    check!(eq; first, second);
    check!(eq; first.established_ids, vec![PublicAthleteId::parse(FIRST)?]);
    Ok(())
}

#[test]
fn exact_retained_event_replay_preserves_id_and_binding_positions() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    let bytes = serde_json::to_vec(&fixture.records)?;
    for projection in [fixture.project()?, fixture.project()?] {
        check!(eq; projection.resolve_candidate(&candidate("z"))?,
            PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
        check!(eq; projection.head(), &fixture.head);
        check!(eq; projection.public_ids().count(), 1);
    }
    check!(eq; serde_json::to_vec(&fixture.records)?, bytes);
    Ok(())
}

#[test]
fn appending_the_same_group_authority_again_is_refused() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(group())?;
    let _second = fixture.append(group())?;
    check!(eq; fixture.project().err(), Some(HistoryError::Duplicate("application")));
    Ok(())
}

#[test]
fn reused_authority_with_different_members_is_refused() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(group())?;
    let _second = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("b", "3"), binding("c", "4")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["b", "c"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::Duplicate("application")));
    Ok(())
}

#[test]
fn evidence_must_cover_exactly_the_admitted_group() -> TestResult {
    for members in [&["a"][..], &["a", "b"][..], &["a", "z", "b"][..]] {
        let mut fixture = Fixture::new()?;
        let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
            bindings: vec![binding("a", "1"), binding("z", "2")],
            evidence: evidence("matched:wrong", CandidateEvidenceKind::SamePerson, members),
        })?;
        check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    }
    Ok(())
}

#[test]
fn source_bound_and_correction_references_cannot_establish_a_matched_group() -> TestResult {
    for kind in [
        CandidateEvidenceKind::SourceBound,
        CandidateEvidenceKind::Correction,
    ] {
        let mut fixture = Fixture::new()?;
        let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
            bindings: vec![binding("a", "1"), binding("z", "2")],
            evidence: evidence("matched:wrong-kind", kind, &["a", "z"]),
        })?;
        check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    }
    Ok(())
}

#[test]
fn a_matched_group_cannot_reestablish_an_existing_candidate() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "old"))?;
    let mut already_bound = binding("a", "1");
    already_bound.id = "binding:a:second".into();
    let _second = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![already_bound, binding("z", "2")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::BindingConflict));
    Ok(())
}

#[test]
fn duplicate_binding_ids_in_one_group_are_refused() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut duplicate = binding("z", "2");
    duplicate.id = "binding:a".into();
    let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), duplicate],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::Duplicate("binding ID")));
    Ok(())
}

#[test]
fn duplicate_candidates_do_not_count_as_exact_membership() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut duplicate = binding("a", "2");
    duplicate.id = "binding:a:second".into();
    let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), duplicate],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    Ok(())
}

#[test]
fn repeated_source_object_in_one_group_is_refused() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), binding("z", "1")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::SourceReuse));
    Ok(())
}

#[test]
fn group_count_is_checked_before_invalid_member_text() -> TestResult {
    let fixture = Fixture::new()?;
    let mut invalid = binding("z", "2");
    invalid.id.clear();
    let request = HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), invalid],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    };
    let limits = ReplayLimits {
        members_per_operation: 1,
        ..ReplayLimits::SMALL
    };
    check!(eq; prepare_next(&fixture.installation, &fixture.head, request, limits).err(),
        Some(HistoryError::Limit("matched group bindings")));
    Ok(())
}

#[test]
fn replay_checks_group_count_before_normalization_or_digest() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    let limits = ReplayLimits {
        members_per_operation: 1,
        ..ReplayLimits::SMALL
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits).err(),
        Some(HistoryError::Limit("matched group bindings")));
    Ok(())
}

#[test]
fn the_whole_group_must_fit_the_remaining_binding_budget() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("b", "3"))?;
    let _second = fixture.append(group())?;
    let limits = ReplayLimits {
        bindings: 2,
        ..ReplayLimits::SMALL
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits).err(),
        Some(HistoryError::Limit("bindings")));
    Ok(())
}

#[test]
fn one_group_uses_exactly_one_identity_budget_slot() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(group())?;
    let limits = ReplayLimits {
        identities: 1,
        bindings: 2,
        ..ReplayLimits::SMALL
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits)?.public_ids().count(), 1);
    let none = ReplayLimits {
        identities: 0,
        ..limits
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, none).err(),
        Some(HistoryError::Limit("identities")));
    Ok(())
}

#[test]
fn empty_and_single_member_groups_are_refused() -> TestResult {
    for bindings in [vec![], vec![binding("a", "1")]] {
        let mut fixture = Fixture::new()?;
        let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
            bindings,
            evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
        })?;
        check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    }
    Ok(())
}

#[test]
fn group_supersession_preserves_the_original_binding_and_explicit_dates() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut old = binding("b", "1");
    old.effective = closed("2020-01-01", "2020-12-31")?;
    let _first = fixture.append(HistoryRequest::Establish {
        binding: old.clone(),
        evidence: evidence("source:b", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    let mut newer = binding("a", "1");
    newer.effective = closed("2021-01-01", "2021-12-31")?;
    newer.supersedes = Some("binding:b".into());
    let _second = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![newer.clone(), binding("z", "2")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    })?;
    let projection = fixture.project()?;
    check!(eq; projection.binding_claim("binding:b"), Some(&old));
    check!(eq; projection.binding_claim("binding:a"), Some(&newer));
    check!(eq; projection.resolve_binding("binding:b")?, BindingResolution::Resolved {
        identity: PublicResolution::Canonical(successor(FIRST, "b", 1, 0)?),
        established_at: 1,
        superseded_by: Some("binding:a".into()),
    });
    check!(eq; projection.resolve_candidate(&candidate("a"))?,
        PublicResolution::Canonical(successor(SECOND, "a", 2, 0)?));
    Ok(())
}

#[test]
fn same_person_membership_does_not_make_unknown_source_reuse_valid() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("b", "1"))?;
    let mut newer = binding("a", "1");
    newer.supersedes = Some("binding:b".into());
    let _second = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![newer, binding("z", "2")],
        evidence: evidence("matched:az", CandidateEvidenceKind::SamePerson, &["a", "z"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::SourceReuse));
    Ok(())
}
