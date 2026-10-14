use super::fixture::*;
use super::*;

fn revoke(merges: Vec<u64>, reference: &str) -> TestResult<HistoryRequest> {
    Ok(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: merges,
        affected_record_keys: vec!["alias:a".into(), "alias:b".into(), "alias:c".into()],
        evidence: evidence(
            reference,
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    }))
}

fn triangle() -> TestResult<Fixture> {
    let mut fixture = chain()?;
    fixture.merge(FIRST, &[FIRST], &["a", "c"], "merge:ac")?;
    Ok(fixture)
}

#[test]
fn independent_connection_inside_an_existing_component_is_retained() -> TestResult {
    let mut fixture = triangle()?;
    let births = fixture.append(revoke(vec![4], "revoke:ab")?)?;
    check!(births.is_empty());
    let projection = fixture.project()?;
    check!(eq; projection.public_ids().count(), 3);
    check!(eq; projection.revoked_merges().collect::<Vec<_>>(), vec![(4, 7)]);
    check!(eq; projection.correction_history(&PublicAthleteId::parse(FIRST)?), Some([4].as_slice()));
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?,
        PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?, PublicResolution::RevokedRedirect {
        successor: successor(FIRST, "a", 1, 0)?, changed_at: 7, revoked_merges: vec![4],
    });
    Ok(())
}

#[test]
fn bridge_revocation_requires_explicit_split_partitions() -> TestResult {
    let mut fixture = chain()?;
    let _births = fixture.append(revoke(vec![4], "revoke:ab")?)?;
    check!(eq; fixture.project().err(), Some(HistoryError::SplitRequired));
    Ok(())
}

#[test]
fn repeated_exact_revocation_is_not_a_new_correction() -> TestResult {
    let mut fixture = triangle()?;
    let _first = fixture.append(revoke(vec![4], "revoke:first")?)?;
    let _second = fixture.append(revoke(vec![4], "revoke:second")?)?;
    check!(eq; fixture.project().err(), Some(HistoryError::Revocation));
    Ok(())
}

#[test]
fn removing_another_edge_can_disconnect_a_previously_corrected_graph() -> TestResult {
    let mut fixture = triangle()?;
    let _first = fixture.append(revoke(vec![4], "revoke:ab")?)?;
    let _second = fixture.append(revoke(vec![5], "revoke:bc")?)?;
    check!(eq; fixture.project().err(), Some(HistoryError::SplitRequired));
    Ok(())
}

#[test]
fn split_cannot_manufacture_a_connection_after_revoking_all_support() -> TestResult {
    let mut fixture = chain()?;
    let _births = fixture.append(HistoryRequest::Split(split_request(
        &[&["a", "b"], &["c"]],
        vec![4, 5],
    )?))?;
    check!(eq; fixture.project().err(), Some(HistoryError::Partition));
    Ok(())
}

#[test]
fn graph_expansion_has_an_explicit_work_ceiling() -> TestResult {
    let mut fixture = triangle()?;
    let _births = fixture.append(revoke(vec![4], "revoke:ab")?)?;
    let limits = ReplayLimits {
        graph_steps: 1,
        ..ReplayLimits::SMALL
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits).err(),
        Some(HistoryError::Limit("graph steps")));
    Ok(())
}

#[test]
fn component_enumeration_refuses_before_collecting_beyond_the_operation_limit() -> TestResult {
    let fixture = chain()?;
    let mut projection = fixture.project()?;
    projection.limits.members_per_operation = 2;
    check!(eq; projection.component_bindings(&PublicAthleteId::parse(FIRST)?).err(),
        Some(HistoryError::Limit("affected bindings")));
    Ok(())
}

#[test]
fn bound_addition_is_an_accepted_connection_that_split_must_revoke() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _bound = fixture.append(HistoryRequest::Bind {
        owner: PublicAthleteId::parse(FIRST)?,
        binding: binding("b", "2"),
        evidence: evidence("match:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    })?;
    let _third = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["b", "c"], "merge:bc")?;
    let split = split_request(&[&["a"], &["b", "c"]], vec![4])?;
    let _births = fixture.append(HistoryRequest::Split(split))?;
    check!(eq; fixture.project().err(), Some(HistoryError::SurvivingMerge));
    Ok(())
}

#[test]
fn canonical_revocation_history_survives_a_later_split() -> TestResult {
    let mut fixture = triangle()?;
    let _revocation = fixture.append(revoke(vec![4], "revoke:ab")?)?;
    let _split = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b", "c"]],
        vec![6],
    )?))?;
    let a = "ath_public_v1_2c752ebd8af336358dd08a36a6dbc2156ccbe4aa01b834b4c7f1fa81a8ec5fb0";
    let b = "ath_public_v1_af3f19b2d1fc1489058d4ae32a35ccd02000dba4de73ef8d66d869828fe8dbeb";
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?, PublicResolution::Ambiguous {
        successors: vec![successor(a, "a", 8, 0)?, successor(b, "b", 8, 1)?],
        unresolved_bindings: false, changed_at: 8, revoked_merges: vec![4, 6],
    });
    check!(eq; projection.correction_history(&PublicAthleteId::parse(a)?), Some([4, 6].as_slice()));
    Ok(())
}

#[test]
fn canonical_revocation_history_survives_merging_into_an_older_person() -> TestResult {
    let mut fixture = Fixture::new()?;
    let fourth = "ath_public_v1_c0dc942c0bf86d0b7d67f40ac8274fda6ded228b911dc9b2824e83decabc6978";
    for (label, source) in [("d", "4"), ("a", "1"), ("b", "2"), ("c", "3")] {
        let _birth = fixture.append(establish(label, source))?;
    }
    fixture.merge(SECOND, &[SECOND, THIRD], &["a", "b"], "merge:ab")?;
    fixture.merge(SECOND, &[SECOND, fourth], &["b", "c"], "merge:bc")?;
    fixture.merge(SECOND, &[SECOND], &["a", "c"], "merge:ac")?;
    let _revocation = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(SECOND)?,
        revoke_merges: vec![5],
        affected_record_keys: vec!["alias:a".into()],
        evidence: evidence(
            "revoke:ab",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    }))?;
    fixture.merge(FIRST, &[FIRST, SECOND], &["d", "a"], "merge:da")?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?, PublicResolution::RevokedRedirect {
        successor: successor(FIRST, "d", 1, 0)?, changed_at: 9, revoked_merges: vec![5],
    });
    check!(eq; projection.correction_history(&PublicAthleteId::parse(FIRST)?), Some([5].as_slice()));
    Ok(())
}

#[test]
fn old_ambiguous_alias_reports_a_successor_correction_revision() -> TestResult {
    let mut fixture = chain()?;
    let _split = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b", "c"]],
        vec![4],
    )?))?;
    fixture.merge(SPLIT_B, &[SPLIT_B], &["b", "c"], "merge:bc-independent")?;
    let _revocation = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(SPLIT_B)?,
        revoke_merges: vec![5],
        affected_record_keys: vec!["alias:bc".into()],
        evidence: evidence(
            "revoke:bc-original",
            CandidateEvidenceKind::Correction,
            &["b", "c"],
        ),
    }))?;
    check!(eq; fixture.project()?.resolve_public(&PublicAthleteId::parse(FIRST)?)?, PublicResolution::Ambiguous {
        successors: vec![successor(SPLIT_B, "b", 6, 1)?, successor(SPLIT_A, "a", 6, 0)?],
        unresolved_bindings: false, changed_at: 8, revoked_merges: vec![4, 5],
    });
    Ok(())
}
