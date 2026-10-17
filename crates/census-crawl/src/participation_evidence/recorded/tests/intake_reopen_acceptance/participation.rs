use super::*;

#[test]
fn genuine_participation_packet_reopens_exact_originals_without_provider_link() -> TestResult {
    check!(eq; hash(STATUS), "2fb479da65fb5cd69a3bb4a6b8a5625c298cfe51cdf06a43efe5359bf31de57a");
    check!(eq; hash(SCHEDULE), "26833e4c45506da5d776eb844b106dcfb041f67f7916bec0fb59c950b99693eb");
    let (directory, store) = fresh_store()?;
    let before = protected_state(&store)?;
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let packet = participation_packet()?;
    let reference = packet.reference()?;
    let payload = packet.payload()?;
    let recording = Recording::new();
    record_independent_participation_packet(&store, &recording, &packet)?;
    check!(eq; protected_state(&store)?, before);
    let recorded = recording.drain();
    check!(eq; recorded, expected_journal(PARTICIPATION_EVIDENCE_PHASE, reference.as_str(), payload.clone()));
    let prepared = crate::recording::prepare_recorded(recorded, "recorded-participation-reopen")?;
    let first = prepared.apply(&store)?;
    check!(eq; first.appended(), 0);
    retained_only_tables(&store, &tables)?;
    drop(store);
    let store = reopen(&directory)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reopened = read_recorded_independent_participation(&snapshot, &reference, &mut grant)?
        .ok_or("participation holder")?;
    exact_root(
        &snapshot,
        reopened.root(),
        RetainedCaptureRootSchemaV1::ParticipationPacketV1,
        PARTICIPATION_EVIDENCE_PHASE,
        reference.as_str(),
        &payload,
    )?;
    exact_capture(&payload, Some("status"), STATUS)?;
    exact_capture(&payload, Some("schedule"), SCHEDULE)?;
    let checked = reopened.checked();
    check!(eq; checked.reference(), &reference);
    check!(eq; checked.store_identity(), &snapshot.store_identity()?);
    check!(eq; checked.snapshot_sequence(), snapshot.sequence());
    check!(eq; checked.subject().name(), "Zacchaeus Brocks");
    check!(eq; checked.subject().school(), "Detroit Catholic Central");
    check!(eq; checked.day().as_str(), "2026-05-15");
    check!(eq; checked.school_year().get(), 2025);
    check!(eq; checked.cohort().get(), 2026);
    check!(eq; checked.competition(), "MHSAA Regionals");
    check!(eq; checked.venue(), "Milford HS");
    check!(eq; checked.status_proof().capture(), hash(STATUS));
    check!(eq; checked.season_proof().capture(), hash(SCHEDULE));
    let SubjectOwnershipV1::DocumentOnly(subject) = checked.ownership() else {
        return Err("original document gained an unsupported provider owner".into());
    };
    check!(eq; subject, checked.subject());
    let installed = protected_state(&store)?;
    let repeated = prepared.apply(&store)?;
    check!(eq; repeated.appended(), 0);
    check!(eq; repeated.receipt(), first.receipt());
    check!(eq; protected_state(&store)?, installed);
    retained_only_tables(&store, &tables)?;
    no_public_birth(&store)?;
    Ok(())
}
