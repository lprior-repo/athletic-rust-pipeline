use super::fixture::*;
use super::*;

fn attachment(label: &str) -> TestResult<HistoryRequest> {
    Ok(HistoryRequest::BindCandidateMembership {
        owner: PublicAthleteId::parse(FIRST)?,
        membership: CandidateMembershipClaimV1 {
            id: format!("membership:{label}"),
            candidate: candidate(label),
            anchor_binding_id: "binding:a".into(),
            source: binding("a", "1").source,
        },
        evidence: evidence(
            &format!("attach:{label}"),
            CandidateEvidenceKind::SamePerson,
            &["a", label],
        ),
    })
}

fn attached() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    let _birth = fixture.append(establish("a", "1"))?;
    check!(fixture.append(attachment("b")?)?.is_empty());
    Ok(fixture)
}

fn withhold() -> TestResult<HistoryRequest> {
    Ok(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![2],
        affected_record_keys: vec!["athlete:a".into(), "athlete:b".into()],
        evidence: evidence("withhold:b", CandidateEvidenceKind::Correction, &["a", "b"]),
    }))
}

#[test]
fn source_membership_preserves_owner_representative_and_original_binding() -> TestResult {
    let fixture = attached()?;
    let projection = fixture.project()?;
    check!(eq; projection.public_ids().count(), 1);
    check!(eq; projection.resolve_candidate(&candidate("b"))?, PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(projection.binding_claim("membership:b").is_none());
    check!(projection.membership_claim("membership:b").is_some());
    check!(eq; projection.binding_claim("binding:a"), Some(&binding("a", "1")));
    Ok(())
}

#[test]
fn withholding_detaches_only_membership_and_cannot_reapply_candidate() -> TestResult {
    let mut fixture = attached()?;
    check!(fixture.append(withhold()?)?.is_empty());
    let projection = fixture.project()?;
    check!(eq; projection.resolve_candidate(&candidate("b"))?, PublicResolution::Unresolved);
    check!(eq; projection.resolve_candidate(&candidate("a"))?, PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(eq; projection.withheld_memberships(3), Some(["membership:b".to_owned()].as_slice()));
    let mut repeated = attachment("b")?;
    if let HistoryRequest::BindCandidateMembership {
        membership,
        evidence,
        ..
    } = &mut repeated
    {
        membership.id = "membership:b-reissued".into();
        evidence.application_id = "attach:b-reissued".into();
    }
    let _births = fixture.append(repeated)?;
    check!(eq; fixture.project().err(), Some(HistoryError::BindingConflict));
    Ok(())
}

#[test]
fn independent_accepted_edge_retains_membership_after_attachment_revocation() -> TestResult {
    let mut fixture = attached()?;
    fixture.merge(FIRST, &[FIRST], &["a", "b"], "independent:ab")?;
    let _births = fixture.append(withhold()?)?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_candidate(&candidate("b"))?, PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(eq; projection.withheld_memberships(4), Some([].as_slice()));
    Ok(())
}

#[test]
fn copied_source_and_membership_anchor_chains_refuse() -> TestResult {
    for chained in [false, true] {
        let mut fixture = attached()?;
        let mut request = attachment("c")?;
        if let HistoryRequest::BindCandidateMembership { membership, .. } = &mut request {
            if chained {
                membership.anchor_binding_id = "membership:b".into();
            } else {
                membership.source.identifier = "foreign".into();
            }
        }
        let _births = fixture.append(request)?;
        check!(fixture.project().is_err());
    }
    Ok(())
}

#[test]
fn candidate_and_claim_namespaces_cannot_be_reused_as_sources() -> TestResult {
    let mut fixture = attached()?;
    let _births = fixture.append(establish("b", "foreign"))?;
    check!(eq; fixture.project().err(), Some(HistoryError::BindingConflict));
    let mut fixture = attached()?;
    let mut request = establish("c", "foreign");
    if let HistoryRequest::Establish { binding, .. } = &mut request {
        binding.id = "membership:b".into();
    }
    let _births = fixture.append(request)?;
    check!(eq; fixture.project().err(), Some(HistoryError::Duplicate("binding ID")));
    Ok(())
}

#[test]
fn membership_consumes_combined_claim_and_correctable_group_limits() -> TestResult {
    let fixture = attached()?;
    for limits in [
        ReplayLimits {
            bindings: 1,
            ..ReplayLimits::SMALL
        },
        ReplayLimits {
            members_per_operation: 1,
            ..ReplayLimits::SMALL
        },
    ] {
        check!(replay_candidate_history(
            &fixture.installation,
            &fixture.head,
            &fixture.records,
            limits
        )
        .is_err());
    }
    let limits = ReplayLimits {
        bindings: 2,
        members_per_operation: 2,
        ..ReplayLimits::SMALL
    };
    check!(replay_candidate_history(
        &fixture.installation,
        &fixture.head,
        &fixture.records,
        limits
    )
    .is_ok());
    Ok(())
}

#[test]
fn split_cannot_assign_membership_away_from_its_source_anchor() -> TestResult {
    let mut fixture = attached()?;
    let _birth = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST, THIRD], &["a", "c"], "merge:ac")?;
    let mut split = split_request(&[&["a"], &["b", "c"]], vec![2, 4])?;
    for partition in &mut split.partitions {
        if partition.representative == candidate("b") {
            partition.representative = candidate("c");
        }
        for id in &mut partition.binding_ids {
            if id == "binding:b" {
                *id = "membership:b".into();
            }
        }
    }
    let _births = fixture.append(HistoryRequest::Split(split))?;
    check!(eq; fixture.project().err(), Some(HistoryError::Invalid("source epoch evidence required")));
    Ok(())
}

mod graph;
mod historical_replay;
