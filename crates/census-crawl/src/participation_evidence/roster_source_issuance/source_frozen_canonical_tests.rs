use super::*;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;

fn witness(
    claim: &ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Value> {
    let assessed = assess_reviewed_roster_source_v1(claim, grant)?;
    let school = serde_json::to_value(assessed.school(grant)?)?;
    let program = serde_json::to_value(assessed.program(grant)?)?;
    let (qualification, qualification_bytes) = assessed.qualification_object(grant)?;
    let qualification = json!([qualification, hash(qualification_bytes)]);
    let (activation, activation_bytes) = assessed.activation_object(grant)?;
    let activation = json!([activation, hash(activation_bytes)]);
    let root = claim.evidence(grant)?.retained_root(grant)?.root();
    Ok(
        json!({"school":school,"program":program,"qualification":qualification,
        "activation":activation,"original_root":root}),
    )
}

#[test]
fn trailing_whitespace_reaches_canonicality_check_and_poisons_grant() -> TestResult {
    let (directory, store) = issued()?;
    let source_path = store.root().to_path_buf();
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let expected = witness(
        current.claims(&mut grant)?.first().ok_or("review absent")?,
        &mut grant,
    )?;
    let frozen = current.freeze_closure(&mut grant)?;
    drop(current);
    drop(snapshot);
    drop(store);
    directory.close()?;
    check!(!source_path.exists());

    let mut corrupted = frozen.bytes().to_vec();
    corrupted.push(b' ');
    let mut reference = serde_json::to_value(frozen.reference())?;
    *reference.get_mut("digest").ok_or("reference digest")? = json!(hash(&corrupted));
    *reference.get_mut("bytes").ok_or("reference bytes")? = json!(u64::try_from(corrupted.len())?);
    let reference = serde_json::from_value(reference)?;
    let mut denied = AttachmentReplayGrantV1::new();
    let error = replay_frozen_roster_reviewed_claims_v1(&corrupted, &reference, &mut denied)
        .err()
        .ok_or("noncanonical closure was accepted")?;
    check!(matches!(
        error,
        census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::Corrupt(
                "noncanonical frozen reviewed closure"
            )
        )
    ));
    check!(matches!(
        denied.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));

    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(
        frozen.bytes(),
        frozen.reference(),
        &mut replay_grant,
    )?;
    let actual = witness(
        replay
            .claims(&mut replay_grant)?
            .first()
            .ok_or("replayed review absent")?,
        &mut replay_grant,
    )?;
    check!(eq; actual, expected);
    check!(eq; hash(frozen.bytes()), frozen.digest());
    Ok(())
}
