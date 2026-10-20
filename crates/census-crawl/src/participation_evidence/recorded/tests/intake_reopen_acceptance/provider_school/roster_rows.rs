use super::*;
use crate::milesplit::RosterVerdict;
use crate::participation_evidence::recorded::{
    record_provider_roster_intake, record_provider_roster_rows_v3,
};

#[test]
fn genuine_partial_roster_rows_preserve_rejections_originals_and_receipt_after_reopen() -> TestResult
{
    let (directory, store) = fresh_store()?;
    let packet = packet(DIRECTORY)?;
    let reference = packet.reference()?;
    let payload = serde_json::to_value(&packet)?;
    let recording = Recording::new();
    let before = protected_state(&store)?;
    let usage = recording.usage();
    check!(record_provider_roster_intake(&store, &recording, &packet).is_err());
    check!(eq; recording.usage(), usage);
    let school = record_provider_school_intake(&store, &recording, &packet)?;
    let checked = record_provider_roster_rows_v3(&store, &recording, &packet)?;
    let RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = checked.roster_verdict()
    else {
        return Err("genuine partial roster was relabeled".into());
    };
    check!(eq; roster.athletes.len(), 146);
    check!(eq; rejected.len(), 18);
    check!(unfinished.is_none());
    check!(rejected
        .iter()
        .all(|row| row.kind == crate::milesplit::RosterRejectionKind::InvalidGraduationYear));
    check!(eq; protected_state(&store)?, before);
    let prepared = crate::recording::prepare_recorded(recording.drain(), "genuine-dcc-partial-v3")?;
    let first = prepared.apply(&store)?;
    let athletes = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; athletes.len(), 146);
    check!(athletes.iter().all(|athlete| athlete.school == school));
    let abbott = athletes
        .iter()
        .find(|athlete| {
            athlete
                .identity_in(&SourceNamespace::MilesplitAthlete)
                .is_some_and(|identity| identity.id == "13749843")
        })
        .ok_or("original Abbott provider identity missing")?;
    check!(eq; abbott.school, school);
    no_public_birth(&store)?;
    drop(store);
    let store = reopen(&directory)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = read_recorded_provider_school(&snapshot, &reference, &mut grant)?
        .ok_or("original provider-school packet missing after reopen")?;
    exact_root(
        &snapshot,
        retained.root(),
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1.phase(),
        reference.as_str(),
        &payload,
    )?;
    exact_capture(&payload, Some("directory"), DIRECTORY)?;
    exact_capture(&payload, Some("roster"), ROSTER)?;
    check!(eq; retained.checked().roster_verdict(), checked.roster_verdict());
    let before_replay = protected_state(&store)?;
    let replayed = prepared.apply(&store)?;
    check!(eq; replayed.appended(), 0);
    check!(eq; replayed.receipt(), first.receipt());
    check!(eq; store.receipt(prepared.operation())?, Some(first.receipt().clone()));
    check!(eq; protected_state(&store)?, before_replay);
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, athletes);
    no_public_birth(&store)?;
    Ok(())
}
