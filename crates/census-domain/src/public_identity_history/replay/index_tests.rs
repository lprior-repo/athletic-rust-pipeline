use super::*;
use crate::model::{ProfileDate, SourceNamespace};
use crate::public_identity_history::*;

#[path = "../tests/fixture.rs"]
mod fixture;
use fixture::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn compare_scans(projection: &HistoryProjection) -> TestResult {
    let mut candidates: BTreeSet<_> = projection
        .claims()
        .map(|(_, candidate, _)| candidate.clone())
        .collect();
    candidates.insert(candidate("absent"));
    for candidate in candidates {
        let scanned: Vec<_> = projection
            .claims()
            .filter(|(_, held, _)| *held == &candidate)
            .collect();
        check!(eq; projection.candidate_claims(&candidate).collect::<Vec<_>>(), scanned);
        let expected = match scanned.first() {
            Some((_, _, Some(owner))) => projection.resolve_public(owner),
            Some((_, _, None)) => Ok(PublicResolution::Unresolved),
            None => Ok(PublicResolution::Unknown),
        };
        check!(eq; projection.resolve_candidate(&candidate), expected);
        let roots = scanned
            .iter()
            .try_fold(BTreeSet::new(), |mut roots, (_, _, owner)| {
                let owner = owner.ok_or(HistoryError::BindingConflict)?;
                roots.insert(projection.root(owner)?);
                Ok::<_, HistoryError>(roots)
            });
        let expected_root = roots.and_then(|roots| {
            if roots.len() != 1 {
                return Err(HistoryError::BindingConflict);
            }
            roots
                .into_iter()
                .next()
                .ok_or(HistoryError::BindingConflict)
        });
        check!(eq; projection.candidate_root(&candidate), expected_root);
    }
    let mut sources: BTreeSet<_> = projection
        .bindings
        .values()
        .map(|entry| entry.claim.source.clone())
        .collect();
    sources.insert(SourceObjectKey {
        namespace: SourceNamespace::TfrrsAthlete,
        identifier: "absent".into(),
    });
    for source in sources {
        let scanned: Vec<_> = projection
            .bindings
            .values()
            .filter(|entry| entry.claim.source == source)
            .map(|entry| {
                (
                    &entry.claim,
                    entry.owner.as_ref(),
                    entry.superseded_by.as_ref(),
                )
            })
            .collect();
        let indexed: Vec<_> = projection
            .source_claims(&source)
            .map(|entry| {
                (
                    &entry.claim,
                    entry.owner.as_ref(),
                    entry.superseded_by.as_ref(),
                )
            })
            .collect();
        check!(eq; indexed, scanned);
        for day in ["2020-06-01", "2026-06-01", "2027-06-01"] {
            let date = ProfileDate::parse(day)?;
            let expected = projection
                .bindings
                .values()
                .filter(|entry| entry.claim.source == source)
                .try_fold(Vec::new(), |mut ids, entry| {
                    match &entry.claim.effective {
                        EffectiveScope::Unknown => return Err(HistoryError::BindingConflict),
                        EffectiveScope::Closed(range) if range.contains(&date) => {
                            ids.push(entry.claim.id.as_str());
                        }
                        EffectiveScope::Closed(_) => {}
                    }
                    Ok(ids)
                });
            check!(eq; projection.source_bindings_at(&source, &date), expected);
        }
    }
    check!(eq; projection.candidate_bindings.values().map(BTreeSet::len).sum::<usize>(), projection.bindings.len());
    check!(eq; projection.candidate_memberships.values().map(BTreeSet::len).sum::<usize>(), projection.memberships.len());
    check!(eq; projection.source_bindings.values().map(BTreeSet::len).sum::<usize>(), projection.bindings.len());
    Ok(())
}

fn compare_prefixes(fixture: &Fixture) -> TestResult {
    let initial = genesis(&fixture.installation)?;
    let mut projection =
        replay_candidate_history(&fixture.installation, &initial, &[], ReplayLimits::SMALL)?;
    compare_scans(&projection)?;
    for record in &fixture.records {
        projection = projection.extend_candidate(record)?;
        compare_scans(&projection)?;
    }
    check!(eq; projection.head(), &fixture.head);
    compare_scans(&projection.clone())?;
    compare_scans(&fixture.project()?)
}

#[test]
fn indexed_claims_match_scans_after_binding_and_matched_establishment() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _born = fixture.append(establish("a", "1"))?;
    let _bound = fixture.append(HistoryRequest::Bind {
        owner: PublicAthleteId::parse(FIRST)?,
        binding: binding("b", "2"),
        evidence: evidence("match:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    })?;
    compare_prefixes(&fixture)?;
    let mut fixture = Fixture::new()?;
    let _born = fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![binding("a", "1"), binding("b", "2")],
        evidence: evidence("group:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    })?;
    compare_prefixes(&fixture)
}

#[test]
fn indexed_membership_owner_remains_live_after_withholding() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _born = fixture.append(establish("a", "1"))?;
    let _attached = fixture.append(HistoryRequest::BindCandidateMembership {
        owner: PublicAthleteId::parse(FIRST)?,
        membership: CandidateMembershipClaimV1 {
            id: "membership:b".into(),
            candidate: candidate("b"),
            anchor_binding_id: "binding:a".into(),
            source: binding("a", "1").source,
        },
        evidence: evidence("attach:b", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    })?;
    let _withheld = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![2],
        affected_record_keys: vec!["athlete:a".into(), "athlete:b".into()],
        evidence: evidence("withhold:b", CandidateEvidenceKind::Correction, &["a", "b"]),
    }))?;
    compare_prefixes(&fixture)
}

#[test]
fn indexed_claims_follow_merge_revocation_and_split_owners() -> TestResult {
    let mut fixture = chain()?;
    fixture.merge(FIRST, &[FIRST], &["a", "c"], "merge:ac")?;
    let _revoked = fixture.append(HistoryRequest::Revoke(RevocationRequest {
        canonical: PublicAthleteId::parse(FIRST)?,
        revoke_merges: vec![4],
        affected_record_keys: vec!["alias:a".into()],
        evidence: evidence(
            "revoke:ab",
            CandidateEvidenceKind::Correction,
            &["a", "b", "c"],
        ),
    }))?;
    compare_prefixes(&fixture)?;
    for unresolved in [false, true] {
        let mut fixture = chain()?;
        let split = if unresolved {
            let mut split = split_request(&[&["b"], &["c"]], vec![4, 5])?;
            split.unresolved_bindings = vec!["binding:a".into()];
            split
        } else {
            split_request(&[&["a"], &["b", "c"]], vec![4])?
        };
        let _split = fixture.append(HistoryRequest::Split(split))?;
        compare_prefixes(&fixture)?;
    }
    let mut fixture = chain()?;
    let _split = fixture.append(HistoryRequest::Split(split_request(
        &[&["a"], &["b"], &["c"]],
        vec![4, 5],
    )?))?;
    compare_prefixes(&fixture)?;
    let projection = fixture.project()?;
    for (label, id, position) in [("a", SPLIT_A, 0), ("b", SPLIT_B, 1), ("c", SPLIT_C, 2)] {
        check!(eq; projection.resolve_candidate(&candidate(label))?,
            PublicResolution::Canonical(successor(id, label, 6, position)?));
    }
    Ok(())
}

#[test]
fn source_index_keeps_all_superseded_epochs_in_binding_id_order() -> TestResult {
    let mut fixture = Fixture::new()?;
    for (label, from, through, supersedes) in [
        ("z", "2020-01-01", "2020-12-31", None),
        ("a", "2026-01-01", "2026-12-31", Some("binding:z")),
        ("m", "2027-01-01", "2027-12-31", Some("binding:a")),
    ] {
        let mut claim = binding(label, "7");
        claim.effective = closed(from, through)?;
        claim.supersedes = supersedes.map(str::to_owned);
        let _born = fixture.append(HistoryRequest::Establish {
            binding: claim,
            evidence: evidence(
                &format!("source:{label}"),
                CandidateEvidenceKind::SourceBound,
                &[label],
            ),
        })?;
    }
    compare_prefixes(&fixture)?;
    let _attached = fixture.append(HistoryRequest::BindCandidateMembership {
        owner: PublicAthleteId::parse(THIRD)?,
        membership: CandidateMembershipClaimV1 {
            id: "membership:new".into(),
            candidate: candidate("new"),
            anchor_binding_id: "binding:m".into(),
            source: binding("m", "7").source,
        },
        evidence: evidence(
            "attach:new",
            CandidateEvidenceKind::SamePerson,
            &["m", "new"],
        ),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::SourceEpochEvidenceRequired));
    Ok(())
}
