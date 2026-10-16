use super::*;

#[test]
fn group_tie_breaking_and_equal_outer_keys_follow_construction_order() -> TestResult {
    let mut same_id_a = binding("a", "1");
    let mut same_id_b = binding("b", "2");
    same_id_a.id = "binding:shared".into();
    same_id_b.id = "binding:shared".into();
    let mut by_candidate = request_order::normalize(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![same_id_a, same_id_b],
        evidence: evidence("candidate-order", CandidateEvidenceKind::SamePerson, &["a"]),
    })?;
    if let HistoryRequest::EstablishMatchedGroup { bindings, .. } = &mut by_candidate {
        bindings.swap(0, 1);
    }
    check!(eq; request_order::require_canonical(&by_candidate),
        Err(HistoryError::Invalid("noncanonical request order")));
    let mut binding_a = binding("a", "1");
    let mut binding_b = binding("b", "2");
    binding_b.candidate = binding_a.candidate.clone();
    let mut group = HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding_b.clone(), binding_a.clone()],
        evidence: evidence("group", CandidateEvidenceKind::SamePerson, &["a"]),
    };
    check!(eq; request_order::require_canonical(&group),
        Err(HistoryError::Invalid("noncanonical request order")));
    binding_a.id = binding_b.id.clone();
    if let HistoryRequest::EstablishMatchedGroup { bindings, .. } = &mut group {
        *bindings = vec![binding_a, binding_b];
    }
    check!(eq; request_order::require_canonical(&group), Ok(()));
    let mut split = split_request(&[&["a"], &["a"]], vec![4])?;
    split.evidence.members = vec![candidate("a")];
    check!(eq; request_order::require_canonical(&HistoryRequest::Split(split)), Ok(()));
    Ok(())
}

#[test]
fn construction_normalizes_a_permutation_without_changing_public_births() -> TestResult {
    let mut fixture = Fixture::new()?;
    let births = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("c", "3"), binding("b", "2"), binding("a", "1")],
        evidence: evidence(
            "group:abc",
            CandidateEvidenceKind::SamePerson,
            &["c", "b", "a"],
        ),
    })?;
    check!(eq; births, vec![PublicAthleteId::parse(FIRST)?]);
    let projection = fixture.project()?;
    check!(eq; projection.head(), &fixture.head);
    check!(eq; projection.public_ids().map(PublicAthleteId::as_str).collect::<Vec<_>>(), vec![FIRST]);
    Ok(())
}

#[test]
fn public_replay_refuses_a_validly_hashed_noncanonical_record() -> TestResult {
    let fixture = chain()?;
    let mut records = fixture.records.clone();
    let last = records.last_mut().ok_or("missing merge record")?;
    let HistoryRequest::Merge { members, .. } = &mut last.request else {
        return Err("expected last fixture record to be a merge".into());
    };
    members.swap(0, 1);
    last.digest = record::payload_digest(last)?;
    let expected_head = RegistryHead {
        sequence: last.sequence,
        digest: last.digest.clone(),
    };
    check!(matches!(
        replay_candidate_history(
            &fixture.installation,
            &expected_head,
            &records,
            ReplayLimits::SMALL
        ),
        Err(HistoryError::Invalid("noncanonical request order"))
    ));
    let valid = fixture.project()?;
    check!(eq; valid.head(), &fixture.head);
    check!(eq; valid.public_ids().map(PublicAthleteId::as_str).collect::<Vec<_>>(), vec![FIRST, THIRD, SECOND]);
    Ok(())
}
