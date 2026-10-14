use super::fixture::*;
use super::*;

#[test]
fn precise_revocation_preserves_an_independent_surviving_merge() -> TestResult {
    let mut fixture = chain()?;
    let original = fixture.records.clone();
    let _births = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b", "c"]],
        vec![4],
    )?))?;
    let projection = fixture.project()?;
    check!(eq; projection.revoked_merges().collect::<Vec<_>>(), vec![(4, 6)]);
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?, PublicResolution::RevokedRedirect {
        successor: successor(SPLIT_B, "b", 6, 1)?, changed_at: 6, revoked_merges: vec![4],
    });
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(THIRD)?)?, PublicResolution::RevokedRedirect {
        successor: successor(SPLIT_B, "b", 6, 1)?, changed_at: 6, revoked_merges: vec![4],
    });
    check!(eq; fixture.records.get(..5), Some(original.as_slice()));
    check!(eq; projection.affected_record_keys(6), Some([String::from("performance:1"), String::from("performance:2")].as_slice()));
    Ok(())
}

#[test]
fn former_merged_canonical_is_an_explicit_ambiguous_alias() -> TestResult {
    let mut fixture = chain()?;
    let _births = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b", "c"]],
        vec![4],
    )?))?;
    check!(eq; fixture.project()?.resolve_public(&PublicAthleteId::parse(FIRST)?)?, PublicResolution::Ambiguous {
        successors: vec![successor(SPLIT_B, "b", 6, 1)?, successor(SPLIT_A, "a", 6, 0)?],
        unresolved_bindings: false, changed_at: 6, revoked_merges: vec![4],
    });
    check!(eq; fixture.project()?.resolve_binding("binding:a")?, BindingResolution::Resolved {
        identity: PublicResolution::Canonical(successor(SPLIT_A, "a", 6, 0)?), established_at: 1, superseded_by: None,
    });
    Ok(())
}

#[test]
fn split_cannot_cut_a_merge_that_was_not_revoked() -> TestResult {
    let mut fixture = chain()?;
    let _births = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b"], &["c"]],
        vec![4],
    )?))?;
    check!(eq; fixture.project().err(), Some(HistoryError::SurvivingMerge));
    Ok(())
}

#[test]
fn revoking_both_connections_allows_three_literal_successors() -> TestResult {
    let mut fixture = chain()?;
    let born = fixture.append(HistoryRequest::Split(split_request(
        &[&["c"], &["a"], &["b"]],
        vec![5, 4],
    )?))?;
    check!(eq; born, vec![PublicAthleteId::parse(SPLIT_A)?, PublicAthleteId::parse(SPLIT_B)?, PublicAthleteId::parse(SPLIT_C)?]);
    check!(eq; fixture.project()?.resolve_public(&PublicAthleteId::parse(SPLIT_C)?)?,
        PublicResolution::Canonical(successor(SPLIT_C, "c", 6, 2)?));
    check!(eq; fixture.project()?.revoked_merges().collect::<Vec<_>>(), vec![(4, 6), (5, 6)]);
    Ok(())
}

#[test]
fn uncertain_binding_is_retained_without_assigning_it_to_a_successor() -> TestResult {
    let mut fixture = chain()?;
    let mut split = split_request(&[&["b"], &["c"]], vec![4, 5])?;
    split.unresolved_bindings = vec!["binding:a".into()];
    let _births = fixture.append(HistoryRequest::Split(split))?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_binding("binding:a")?, BindingResolution::Unresolved);
    check!(eq; projection.resolve_candidate(&candidate("a"))?, PublicResolution::Unresolved);
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?, PublicResolution::Ambiguous {
        successors: vec![successor(SPLIT_B, "c", 6, 1)?, successor(SPLIT_A, "b", 6, 0)?],
        unresolved_bindings: true, changed_at: 6, revoked_merges: vec![4, 5],
    });
    Ok(())
}

#[test]
fn successor_resolution_follows_a_later_explicit_split() -> TestResult {
    let mut fixture = chain()?;
    let _first = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b", "c"]],
        vec![4],
    )?))?;
    let next = SplitRequest {
        canonical: PublicAthleteId::parse(SPLIT_B)?,
        revoke_merges: vec![5],
        partitions: vec![
            BindingPartition {
                binding_ids: vec!["binding:b".into()],
                representative: candidate("b"),
            },
            BindingPartition {
                binding_ids: vec!["binding:c".into()],
                representative: candidate("c"),
            },
        ],
        unresolved_bindings: Vec::new(),
        affected_record_keys: vec!["performance:2".into()],
        evidence: evidence(
            "correction:2",
            CandidateEvidenceKind::Correction,
            &["b", "c"],
        ),
    };
    let _second = fixture.append(HistoryRequest::Split(next))?;
    let b = "ath_public_v1_e41233a5c11d47cba26f08419b5e5ecd724e66e36d63ae1426bcc29c7a361323";
    let c = "ath_public_v1_8bf5ad9c966945c7a0134fddd1749fe9ada0eeafca01fada386852f14b6e291a";
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?, PublicResolution::Ambiguous {
        successors: vec![successor(SPLIT_A, "a", 6, 0)?, successor(c, "c", 7, 1)?, successor(b, "b", 7, 0)?],
        unresolved_bindings: false, changed_at: 7, revoked_merges: vec![4, 5],
    });
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?, PublicResolution::RevokedRedirect {
        successor: successor(b, "b", 7, 0)?, changed_at: 7, revoked_merges: vec![4, 5],
    });
    Ok(())
}

#[test]
fn missing_or_duplicate_partition_members_are_refused() -> TestResult {
    for groups in [
        &[&["a"][..], &["b"][..]][..],
        &[&["a", "b"][..], &["b", "c"][..]][..],
    ] {
        let mut fixture = chain()?;
        let _births = fixture.append(HistoryRequest::Split(split_request(groups, vec![4, 5])?))?;
        check!(eq; fixture.project().err(), Some(HistoryError::Partition));
    }
    Ok(())
}

#[test]
fn unknown_merge_reference_is_not_a_revocation() -> TestResult {
    let mut fixture = chain()?;
    let _births = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b", "c"]],
        vec![99],
    )?))?;
    check!(eq; fixture.project().err(), Some(HistoryError::Unknown("merge event")));
    Ok(())
}
