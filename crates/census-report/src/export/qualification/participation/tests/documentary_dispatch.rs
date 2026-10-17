use super::*;

#[tokio::test]
async fn current_capture_selects_v3_and_historical_v2_replay_keeps_exact_bytes_and_outcomes(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let current = fixture.assess()?;
    check!(matches!(&current.replay, ParticipationReplay::V3(_)));
    let ParticipationReplay::V3(current) = current.replay else {
        return Err("current V3 producer".into());
    };
    let historical = ParticipationReplay::V2(current.retained);
    let expected_wire = serde_json::to_vec(&historical)?;
    let directory = fixture.objects();
    let descriptor = historical.install(&directory)?;
    let before = inventory(&directory)?;
    let loaded = ParticipationReplay::load(&directory, &descriptor)?;
    check!(matches!(&loaded, ParticipationReplay::V2(_)));
    check!(eq; serde_json::to_vec(&loaded)?, expected_wire);
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let replay = PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        &directory,
        &descriptor,
        &historical.fingerprints(),
        affiliations.roster_context(),
    )?;
    check!(replay.documentary.is_empty());
    check!(replay.unsupported_documentary.is_empty());
    let found = replay
        .rows
        .get(&fixture.performance.id)
        .ok_or("historical assessment")?;
    check!(eq; found.outcome, CandidateOutcome::Unresolved);
    check!(eq; found.reasons.iter().collect::<Vec<_>>(), vec![ParticipationReason::HighSchoolStatusMissing, ParticipationReason::SummerBoundaryUnresolved]);
    check!(found.diagnostics.is_empty());
    check!(!found.references.is_empty());
    check!(eq; serde_json::to_vec(&replay.replay)?, expected_wire);
    check!(eq; inventory(&directory)?, before);
    Ok(())
}

#[tokio::test]
async fn replay_v1_dispatch_keeps_wire_and_cannot_issue_documentary_capabilities() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let current = fixture.assess()?;
    let historical: ParticipationReplayV1 =
        serde_json::from_slice(&serde_json::to_vec(current.replay.retained())?)?;
    let expected = serde_json::to_vec(&historical)?;
    let directory = fixture.objects();
    let descriptor = historical.install(&directory)?;
    let before = inventory(&directory)?;
    let loaded = ParticipationReplay::load(&directory, &descriptor)?;
    check!(matches!(&loaded, ParticipationReplay::V1(_)));
    check!(loaded.independent().is_none());
    check!(eq; serde_json::to_vec(&loaded)?, expected);
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    check!(validation::frozen(&dataset, &fixture.profile, &loaded).is_err());
    check!(eq; inventory(&directory)?, before);
    Ok(())
}

#[tokio::test]
async fn v3_missing_or_altered_attachment_never_falls_back_to_valid_nested_v2() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let current = fixture.assess()?;
    let original = serde_json::to_value(&current.replay)?;
    let directory = fixture.objects();
    let nested: ParticipationReplayV2 = serde_json::from_slice(&serde_json::to_vec(
        original.get("retained").ok_or("nested V2")?,
    )?)?;
    let v2 = ParticipationReplay::V2(nested);
    let v2_descriptor = v2.install(&directory)?;
    check!(matches!(
        ParticipationReplay::load(&directory, &v2_descriptor)?,
        ParticipationReplay::V2(_)
    ));
    let mut missing = original.clone();
    missing
        .as_object_mut()
        .ok_or("V3 object")?
        .remove("attachment")
        .ok_or("attachment")?;
    let mut wrong_shape = original.clone();
    *wrong_shape.get_mut("attachment").ok_or("attachment")? = json!({});
    for payload in [missing, wrong_shape] {
        let descriptor = install_wire(&directory, 3, &payload)?;
        let before = inventory(&directory)?;
        check!(ParticipationReplay::load(&directory, &descriptor).is_err());
        check!(eq; inventory(&directory)?, before);
    }
    let mut missing_object = original;
    *missing_object
        .pointer_mut("/attachment/object")
        .ok_or("attachment object digest")? = json!("f".repeat(64));
    let malformed: ParticipationReplayV3 =
        serde_json::from_slice(&serde_json::to_vec(&missing_object)?)?;
    let malformed = ParticipationReplay::V3(malformed).install(&directory)?;
    check!(matches!(
        ParticipationReplay::load(&directory, &malformed)?,
        ParticipationReplay::V3(_)
    ));
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let before = inventory(&directory)?;
    check!(PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        &directory,
        &malformed,
        &current.fingerprints(),
        affiliations.roster_context()
    )
    .is_err());
    check!(eq; inventory(&directory)?, before);
    Ok(())
}

#[tokio::test]
async fn replay_dispatch_refuses_unknown_version_without_mutating_archives() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let current = fixture.assess()?;
    let payload = serde_json::to_value(&current.replay)?;
    let directory = fixture.objects();
    for version in [0, 4, u32::MAX] {
        let descriptor = install_wire(&directory, version, &payload)?;
        let before = inventory(&directory)?;
        check!(ParticipationReplay::load(&directory, &descriptor).is_err());
        check!(eq; inventory(&directory)?, before);
    }
    Ok(())
}

fn install_wire(directory: &Path, version: u32, payload: &Value) -> TestResult<String> {
    let bytes = serde_json::to_vec(payload)?;
    check!(bytes.len() < 4 * 1024 * 1024);
    let chunk = crate::export::proof_objects::install(directory, &bytes)?;
    #[derive(serde::Serialize)]
    struct Descriptor {
        version: u32,
        bytes: usize,
        sha256: String,
        chunks: Vec<String>,
    }
    let descriptor = serde_json::to_vec(&Descriptor {
        version,
        bytes: bytes.len(),
        sha256: archive::hash(&bytes),
        chunks: vec![chunk],
    })?;
    Ok(crate::export::proof_objects::install(
        directory,
        &descriptor,
    )?)
}

fn inventory(directory: &Path) -> TestResult<BTreeMap<std::path::PathBuf, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let path = entry?.path();
            let bytes = std::fs::read(&path)?;
            Ok((path, bytes))
        })
        .collect()
}
