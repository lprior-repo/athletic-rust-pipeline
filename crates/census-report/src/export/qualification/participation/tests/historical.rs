use super::*;

#[tokio::test]
async fn synthetic_historical_v2_frozen_replay_preserves_decision_and_archive_bytes() -> TestResult
{
    historical_roundtrip(
        participation_policy_v2()?,
        CandidateOutcome::EligibleCandidate,
        &[],
    )
    .await
}

#[tokio::test]
async fn synthetic_historical_v3_frozen_replay_preserves_strict_withholding_and_archive_bytes(
) -> TestResult {
    historical_roundtrip(
        participation_policy_v3()?,
        CandidateOutcome::Unresolved,
        &[
            ParticipationReason::HighSchoolStatusMissing,
            ParticipationReason::SummerBoundaryUnresolved,
        ],
    )
    .await
}

async fn historical_roundtrip(
    rule: census_domain::model::ContractRefV1,
    expected: CandidateOutcome,
    expected_reasons: &[ParticipationReason],
) -> TestResult {
    let mut fixture = Fixture::prepare(Some("High School")).await?;
    fixture.profile = profile::profile(rule)?;
    fixture.store.register_profile(&fixture.profile)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    assert!(validation::live(&dataset, &snapshot, &fixture.profile).is_err());
    let directory = fixture.objects();
    std::fs::create_dir_all(&directory)?;
    let athletes = crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &directory,
        observations::BUDGET,
    )?;
    let (closure, observations) = observations::capture(
        &dataset,
        &snapshot,
        &directory,
        &athletes,
        athletes.fingerprint(),
    )?;
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let captured_at = DateTime::parse_from_rfc3339("2026-10-15T14:00:00Z")?.with_timezone(&Utc);
    let recorder = RecordingRetainedJournal::new(&snapshot);
    let historical_rows = assess(
        &dataset,
        &fixture.profile,
        &recorder,
        &closure,
        captured_at,
        affiliations.roster_context(),
        None,
    )?;
    let historical_row = historical_rows
        .get(&fixture.performance.id)
        .ok_or("historical row")?;
    assert_eq!(historical_row.outcome, expected);
    assert_eq!(
        historical_row.reasons.iter().collect::<Vec<_>>(),
        expected_reasons
    );
    let historical = ParticipationReplayV1 {
        version: 1,
        store_identity: dataset.lineage.store_identity.clone(),
        snapshot_sequence: snapshot.sequence(),
        captured_at,
        input_generation: dataset.lineage.input_generation.clone(),
        input_digest: dataset.lineage.input_digest.clone(),
        source_digest: dataset.lineage.source_digest.clone(),
        profile_id: fixture.profile.id()?,
        profile: fixture.profile.clone(),
        observations,
        journal: recorder.finish()?,
    };
    let descriptor = historical.install(&directory)?;
    let historical_wire = serde_json::to_vec(&historical)?;
    let bytes_before = archive_bytes(&directory)?;
    let replay = PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        &directory,
        &descriptor,
        &historical.fingerprints(),
        affiliations.roster_context(),
    )?;
    assert_eq!(serde_json::to_vec(&replay.replay)?, historical_wire);
    let (outcome, reasons, references, gaps) = row(replay, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, expected);
    assert_eq!(reasons.iter().collect::<Vec<_>>(), expected_reasons);
    assert!(gaps.is_empty());
    assert!(!references.is_empty());
    assert_eq!(archive_bytes(&directory)?, bytes_before);
    assert!(fixture.read(&dataset, &snapshot, &fixture.profile).is_err());
    assert_eq!(archive_bytes(&directory)?, bytes_before);
    Ok(())
}

fn archive_bytes(directory: &Path) -> TestResult<BTreeMap<std::path::PathBuf, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let path = entry?.path();
            let bytes = std::fs::read(&path)?;
            Ok((path, bytes))
        })
        .collect()
}
