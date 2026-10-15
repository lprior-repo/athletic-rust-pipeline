use super::*;
use serde::de::DeserializeSeed;

#[test]
fn historical_revoke_does_not_charge_a_second_completed_graph_walk() -> TestResult {
    replay_at_historical_budget(3_700, 2, 500_144)
}

#[test]
fn historical_revoke_validates_surviving_edges_inside_the_existing_work_budget() -> TestResult {
    replay_at_historical_budget(1_800, 128, 924_170)
}

#[test]
fn completed_membership_walk_still_refuses_a_surviving_cross_component_edge() -> TestResult {
    let mut fixture = attached()?;
    let _births = fixture.append(establish("c", "3"))?;
    fixture.merge(FIRST, &[FIRST], &["a", "b"], "independent:ab")?;
    let mut projection = fixture.project()?;
    projection.merges.insert(
        5,
        crate::public_identity_history::replay::MergeEntry {
            binding_ids: ["binding:a".to_owned(), "binding:c".to_owned()]
                .into_iter()
                .collect(),
            revoked_at: None,
        },
    );
    check!(eq; projection.preview_withheld_memberships(&PublicAthleteId::parse(FIRST)?, 2).err(), Some(HistoryError::SurvivingMerge));
    Ok(())
}

#[test]
fn historical_singleton_split_partitions_keep_zero_traversal_cost() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), binding("b", "2")],
        evidence: evidence("matched:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    })?;
    let _births = fixture.append(HistoryRequest::Split(SplitRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![1],
        partitions: ["a", "b"]
            .into_iter()
            .map(|label| BindingPartition {
                binding_ids: vec![format!("binding:{label}")],
                representative: candidate(label),
            })
            .collect(),
        unresolved_bindings: vec![],
        affected_record_keys: vec!["athlete:group".into()],
        evidence: evidence("split:ab", CandidateEvidenceKind::Correction, &["a", "b"]),
    }))?;
    let limits = ReplayLimits {
        graph_steps: 14,
        ..ReplayLimits::SMALL
    };
    let projection = replay_candidate_history(
        &fixture.installation,
        &fixture.head,
        &fixture.records,
        limits,
    )?;
    check!(
        projection.resolve_candidate(&candidate("a"))?
            != projection.resolve_candidate(&candidate("b"))?
    );
    check!(eq; projection.head(), &fixture.head);
    Ok(())
}

fn replay_at_historical_budget(
    redundant_merges: usize,
    members_per_merge: usize,
    graph_steps: usize,
) -> TestResult {
    let fixture = historical_history(redundant_merges, members_per_merge)?;
    let raw = serde_json::to_vec(&fixture.records)?;
    check!(raw.len() <= 16 * 1024 * 1024);
    let limits = ReplayLimits {
        graph_steps,
        ..ReplayLimits::SMALL
    };
    let mut parser = serde_json::Deserializer::from_slice(&raw);
    let decoded = CandidateHistorySeed::new(&fixture.installation, &fixture.head, limits)
        .deserialize(&mut parser)?;
    parser.end()?;
    check!(eq; decoded.records(), fixture.records.as_slice());
    check!(eq; serde_json::to_vec(decoded.records())?, raw);
    check!(eq; decoded.projection().head(), &fixture.head);
    check!(eq; decoded.projection().withheld_memberships(fixture.head.sequence), Some([].as_slice()));
    let lower = ReplayLimits {
        graph_steps: graph_steps.checked_sub(1).ok_or("empty graph budget")?,
        ..limits
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, lower).err(), Some(HistoryError::Limit("graph steps")));
    Ok(())
}

fn historical_history(redundant_merges: usize, members_per_merge: usize) -> TestResult<Fixture> {
    let members = (0..128)
        .map(|index| {
            serde_json::from_value::<crate::model::AthleteCandidateId>(serde_json::json!(format!(
                "c{index:03}"
            )))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bindings = members
        .iter()
        .map(|candidate| BindingClaim {
            id: format!("binding:{}", candidate.as_str()),
            source: SourceObjectKey {
                namespace: crate::model::SourceNamespace::TfrrsAthlete,
                identifier: candidate.as_str().into(),
            },
            candidate: candidate.clone(),
            effective: EffectiveScope::Unknown,
            supersedes: None,
        })
        .collect();
    let mut fixture = Fixture::new()?;
    let births = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings,
        evidence: historical_reference("group", CandidateEvidenceKind::SamePerson, &members),
    })?;
    let canonical = births.first().ok_or("missing group identity")?.clone();
    let edge_members = members
        .iter()
        .take(members_per_merge)
        .cloned()
        .collect::<Vec<_>>();
    for position in 0..redundant_merges {
        let _births = fixture.append(HistoryRequest::Merge {
            canonical: canonical.clone(),
            members: vec![canonical.clone()],
            evidence: historical_reference(
                &format!("redundant:{position}"),
                CandidateEvidenceKind::SamePerson,
                &edge_members,
            ),
        })?;
    }
    let _births = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical,
        revoke_merges: vec![2],
        affected_record_keys: vec!["athlete:group".into()],
        evidence: historical_reference("revoke", CandidateEvidenceKind::Correction, &members),
    }))?;
    Ok(fixture)
}

fn historical_reference(
    application_id: &str,
    kind: CandidateEvidenceKind,
    members: &[crate::model::AthleteCandidateId],
) -> CandidateDecisionReference {
    CandidateDecisionReference {
        application_id: application_id.into(),
        evidence_digest: "a".repeat(64),
        kind,
        members: members.to_vec(),
    }
}
