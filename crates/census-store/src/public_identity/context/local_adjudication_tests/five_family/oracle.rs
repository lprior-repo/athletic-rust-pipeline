use super::*;
use census_domain::model::{serialized_digest, AthleteCandidateId, SourceObservation};
use census_domain::public_identity_history::local_adjudication::{
    RetainedClaimV1, RetainedIdentitySectionV1,
};
use census_domain::public_identity_history::{
    EstablishmentOrdinal, PublicAthleteId, ResolvedSuccessor,
};

type PhysicalEntityRow = (Vec<u8>, Vec<u8>, u64);

pub(super) fn claims(fixture: &ReviewedFixture, request: &LocalIdentitySplitV1) -> TestResult {
    let athletes = fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; athletes.len(), 5);
    let observations =
        results::raw::<SourceObservation>(&fixture.store, Table::SourceObservations)?;
    check!(eq; observations.len(), 2);
    let mut members = fixture
        .candidates
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    members.sort();
    let mut expected = Vec::new();
    for candidate in members {
        let (ordinal, athlete) = athletes
            .iter()
            .enumerate()
            .find(|(_, row)| row.id.as_str() == candidate.as_str())
            .ok_or("source athlete")?;
        let value = serde_json::to_value(athlete)?;
        let source_links = if candidate == fixture.candidates[0] {
            "/source_links/0"
        } else {
            "/source_links"
        };
        for path in [
            "/canonical_name",
            "/evidence/0",
            "/gender",
            "/grad_year",
            "/id",
            "/identity_attestations/0",
            "/known_names/0",
            "/observed_grades",
            "/public_profile_urls",
            "/school",
            "/source",
            source_links,
            "/sports",
        ] {
            let retained = value
                .pointer(path)
                .ok_or("fixed fixture claim path")?
                .clone();
            expected.push(RetainedClaimV1 {
                candidate: candidate.clone(),
                section: RetainedIdentitySectionV1::Athlete,
                ordinal: u64::try_from(ordinal)?,
                row_digest: serialized_digest(athlete)?,
                claim_path: path.into(),
                claim_digest: serialized_digest(&retained)?,
                retained_value: retained,
            });
        }
        if candidate == fixture.candidates[0] {
            let (ordinal, observation) = observations
                .iter()
                .enumerate()
                .find(|(_, row)| {
                    matches!(row,
                SourceObservation::Athlete(row) if row.source_athlete_id == "9201")
                })
                .ok_or("original source observation")?;
            let value = serde_json::to_value(observation)?;
            expected.push(RetainedClaimV1 {
                candidate,
                section: RetainedIdentitySectionV1::SourceObservation,
                ordinal: u64::try_from(ordinal)?,
                row_digest: serialized_digest(observation)?,
                claim_path: String::new(),
                claim_digest: serialized_digest(&value)?,
                retained_value: value,
            });
        }
    }
    check!(eq; expected.len(), 27);
    check!(eq; request.dispositions.iter().map(|row| &row.claim).collect::<Vec<_>>(), expected.iter().collect::<Vec<_>>());
    for disposition in &request.dispositions {
        let expected = if disposition.claim.claim_path == "/source_links/0"
            || disposition.claim.claim_path == "/identity_attestations/0"
        {
            ClaimTreatmentV1::RejectIdentityCorroboration
        } else {
            ClaimTreatmentV1::RetainAsContext
        };
        check!(eq; disposition.treatment, expected);
    }
    Ok(())
}

pub(super) fn withholding_claims(
    fixture: &ReviewedFixture,
    attachment: &SourceCandidateMembershipRequestV1,
    history_digest: &str,
    claims: &[RetainedClaimV1],
) -> TestResult<String> {
    use census_domain::model::SourceNamespace;
    use census_domain::public_identity_history::{CandidateMembershipClaimV1, SourceObjectKey};
    let digest = serialized_digest(&(
        "public-identity-source-membership-request-v1",
        fixture.store.store_identity()?,
        "five-membership",
        serde_json::to_vec(attachment)?,
    ))?;
    let id = format!("source_membership_v1_{digest}");
    let membership = CandidateMembershipClaimV1 {
        id: id.clone(),
        candidate: attachment.candidate.clone(),
        anchor_binding_id: attachment.anchor_binding_id.clone(),
        source: SourceObjectKey {
            namespace: SourceNamespace::MilesplitAthlete,
            identifier: "9204".into(),
        },
    };
    let athletes = fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; athletes.len(), 5);
    let observations =
        results::raw::<SourceObservation>(&fixture.store, Table::SourceObservations)?;
    check!(eq; observations.len(), 2);
    let mut members = vec![fixture.candidates[2].clone(), attachment.candidate.clone()];
    members.sort();
    let mut expected = Vec::new();
    for candidate in members {
        let (ordinal, athlete) = athletes
            .iter()
            .enumerate()
            .find(|(_, row)| row.id.as_str() == candidate.as_str())
            .ok_or("membership athlete")?;
        let value = serde_json::to_value(athlete)?;
        check!(athlete.identity_attestations.is_empty());
        check!(value.get("identity_attestations").is_none());
        for path in [
            "/canonical_name",
            "/evidence/0",
            "/gender",
            "/grad_year",
            "/id",
            "/known_names/0",
            "/observed_grades",
            "/public_profile_urls",
            "/school",
            "/source",
            "/source_links",
            "/sports",
        ] {
            let retained = value
                .pointer(path)
                .ok_or("fixed membership claim path")?
                .clone();
            expected.push(RetainedClaimV1 {
                candidate: candidate.clone(),
                section: RetainedIdentitySectionV1::Athlete,
                ordinal: u64::try_from(ordinal)?,
                row_digest: serialized_digest(athlete)?,
                claim_path: path.into(),
                claim_digest: serialized_digest(&retained)?,
                retained_value: retained,
            });
        }
        let (ordinal, observation) = observations
            .iter()
            .enumerate()
            .find(|(_, row)| {
                matches!(row,
            SourceObservation::Athlete(row) if row.source_athlete_id == "9204")
            })
            .ok_or("membership source observation")?;
        let value = serde_json::to_value(observation)?;
        expected.push(RetainedClaimV1 {
            candidate,
            section: RetainedIdentitySectionV1::SourceObservation,
            ordinal: u64::try_from(ordinal)?,
            row_digest: serialized_digest(observation)?,
            claim_path: String::new(),
            claim_digest: serialized_digest(&value)?,
            retained_value: value,
        });
    }
    let value = serde_json::to_value(&membership)?;
    expected.push(RetainedClaimV1 {
        candidate: attachment.candidate.clone(),
        section: RetainedIdentitySectionV1::CandidateMembership,
        ordinal: 5,
        row_digest: history_digest.into(),
        claim_path: id.clone(),
        claim_digest: serialized_digest(&value)?,
        retained_value: value,
    });
    check!(eq; expected.len(), 27);
    check!(eq; claims, expected);
    Ok(id)
}

fn public_id(store: &Store, sequence: u64, position: u32) -> TestResult<PublicAthleteId> {
    #[derive(serde::Serialize)]
    struct ExpectedOrdinal {
        sequence: u64,
        position: u32,
    }
    let ordinal = ExpectedOrdinal { sequence, position };
    let digest = serialized_digest(&("public-athlete-id-v1", store.store_identity()?, ordinal))?;
    Ok(PublicAthleteId::parse(&format!("ath_public_v1_{digest}"))?)
}

fn successor(
    store: &Store,
    candidate: &AthleteCandidateId,
    sequence: u64,
    position: u32,
) -> TestResult<ResolvedSuccessor> {
    Ok(ResolvedSuccessor {
        public_id: public_id(store, sequence, position)?,
        representative_candidate: candidate.clone(),
        established: EstablishmentOrdinal { sequence, position },
    })
}

pub(super) fn checkpoints(
    fixture: &ReviewedFixture,
    snapshots: &[(AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2)],
    attachment: &SourceCandidateMembershipRequestV1,
    manual_keys: &[String],
    withheld_keys: &[String],
) -> TestResult {
    let a = successor(&fixture.store, &fixture.candidates[0], 1, 0)?;
    let b = successor(&fixture.store, &fixture.candidates[1], 2, 0)?;
    let unrelated = successor(&fixture.store, &fixture.candidates[2], 3, 0)?;
    let mut partitions = Vec::new();
    for (candidate, operation) in [
        (&fixture.candidates[0], "reviewed-first-birth"),
        (&fixture.candidates[1], "reviewed-second-birth"),
    ] {
        let digest = serialized_digest(&(
            "public-identity-establish-request-v1",
            fixture.store.store_identity()?,
            operation,
            candidate,
        ))?;
        partitions.push((format!("source_binding_v1_{digest}"), candidate));
    }
    partitions.sort_by(|left, right| left.0.cmp(&right.0));
    let split = partitions
        .iter()
        .enumerate()
        .map(|(position, (_, candidate))| {
            successor(&fixture.store, candidate, 7, u32::try_from(position)?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let mut ordered_split = split.clone();
    ordered_split.sort_by(|left, right| left.public_id.cmp(&right.public_id));
    for (snapshot, _) in snapshots {
        let sequence = snapshot.head().sequence;
        let mut ids = vec![
            a.public_id.clone(),
            b.public_id.clone(),
            unrelated.public_id.clone(),
        ];
        if sequence >= 7 {
            ids.extend(split.iter().map(|row| row.public_id.clone()));
        }
        ids.sort();
        check!(eq; snapshot.public_ids().cloned().collect::<Vec<_>>(), ids);
        let old_a = if sequence >= 7 {
            PublicResolution::Ambiguous {
                successors: ordered_split.clone(),
                unresolved_bindings: false,
                changed_at: 7,
                revoked_merges: vec![4],
            }
        } else {
            PublicResolution::Canonical(a.clone())
        };
        let target_b = split
            .iter()
            .find(|row| row.representative_candidate == fixture.candidates[1])
            .ok_or("B successor")?;
        let old_b = if sequence >= 7 {
            PublicResolution::RevokedRedirect {
                successor: target_b.clone(),
                changed_at: 7,
                revoked_merges: vec![4],
            }
        } else if sequence >= 4 {
            PublicResolution::Redirect {
                successor: a.clone(),
                changed_at: 4,
            }
        } else {
            PublicResolution::Canonical(b.clone())
        };
        check!(eq; snapshot.resolve_public(&a.public_id)?, old_a);
        check!(eq; snapshot.resolve_public(&b.public_id)?, old_b);
        check!(eq; snapshot.resolve_public(&unrelated.public_id)?, PublicResolution::Canonical(unrelated.clone()));
        for (index, old) in [(0, old_a), (1, old_b)] {
            let expected = if sequence >= 7 {
                PublicResolution::Canonical(
                    split
                        .iter()
                        .find(|row| row.representative_candidate == fixture.candidates[index])
                        .ok_or("candidate successor")?
                        .clone(),
                )
            } else {
                old
            };
            check!(eq; snapshot.resolve_candidate(&fixture.candidates[index])?, expected);
        }
        check!(eq; snapshot.resolve_candidate(&fixture.candidates[2])?, PublicResolution::Canonical(unrelated.clone()));
        let attached = if sequence >= 6 {
            PublicResolution::Unresolved
        } else if sequence == 5 {
            PublicResolution::Canonical(unrelated.clone())
        } else {
            PublicResolution::Unknown
        };
        check!(eq; snapshot.resolve_candidate(&attachment.candidate)?, attached);
        check!(eq; snapshot.affected_record_keys(6), if sequence >= 6 { Some(withheld_keys) } else { None });
        check!(eq; snapshot.affected_record_keys(7), if sequence >= 7 { Some(manual_keys) } else { None });
        check!(eq; snapshot.correction_history(&a.public_id), if sequence >= 7 { Some([4].as_slice()) } else { Some([].as_slice()) });
        check!(eq; snapshot.correction_history(&unrelated.public_id), if sequence >= 6 { Some([5].as_slice()) } else { Some([].as_slice()) });
    }
    Ok(())
}

pub(super) fn physical(store: &Store, table: Table) -> TestResult<Vec<PhysicalEntityRow>> {
    use fjall::Readable;
    store
        .db
        .snapshot()
        .prefix(&store.entities, crate::keys::observation_prefix(table))
        .enumerate()
        .map(|(ordinal, row)| {
            let (key, value) = row.into_inner()?;
            Ok((key.to_vec(), value.to_vec(), u64::try_from(ordinal)?))
        })
        .collect()
}
