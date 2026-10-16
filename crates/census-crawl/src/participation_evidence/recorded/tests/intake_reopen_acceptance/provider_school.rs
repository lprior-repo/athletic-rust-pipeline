use super::*;
use crate::participation_evidence::provider_school::{
    CheckedProviderSchoolPacketV1, ProviderSchoolPacketRefV1, ProviderSchoolPacketV1,
};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, SourceNamespace, SourceObservation};

mod fixtures;
mod observations;
mod refusals;

use fixtures::{packet, verify_checked_school, DIRECTORY, ROSTER};

#[test]
fn genuine_school_originals_install_before_unaccepted_provider_projection() -> TestResult {
    let (directory, store) = fresh_store()?;
    let packet = packet(DIRECTORY)?;
    let checked = packet.read()?;
    verify_checked_school(&checked)?;
    let reference = packet.reference()?;
    let payload = serde_json::to_value(&packet)?;
    let school = checked.school().clone();
    let recording = Recording::new();
    let before = protected_state(&store)?;
    check!(eq; record_provider_school_intake(&store, &recording, &packet)?, school.id);
    check!(eq; protected_state(&store)?, before);
    let recorded = recording.drain();
    check!(eq; recorded.journal, expected_journal(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1.phase(), reference.as_str(), payload.clone()).journal);
    check!(eq; recorded.rows.len(), 2);
    check!(eq; recorded.effects.len(), 0);
    let prepared = recorded.prepare("genuine-school-originals")?;
    let first = prepared.apply(&store)?;
    check!(eq; first.appended(), 2);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?, vec![school.clone()]);
    check!(eq; store.scan::<SourceObservation>(Table::SourceObservations)?,
        vec![SourceObservation::School(checked.observation().clone())]);
    for table in [
        Table::Athletes,
        Table::Teams,
        Table::Performances,
        Table::SourceIdentities,
    ] {
        check!(eq; store.walk_table(table)?.rows, 0);
    }
    no_public_birth(&store)?;
    drop(store);
    let store = reopen(&directory)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let held = read_recorded_provider_school(&snapshot, &reference, &mut grant)?
        .ok_or("genuine retained school")?;
    exact_root(
        &snapshot,
        held.root(),
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1.phase(),
        reference.as_str(),
        &payload,
    )?;
    exact_capture(&payload, Some("directory"), DIRECTORY)?;
    exact_capture(&payload, Some("roster"), ROSTER)?;
    verify_checked_school(held.checked())?;
    check!(eq; held.checked().school(), &school);
    let before_replay = protected_state(&store)?;
    let replayed = prepared.apply(&store)?;
    check!(eq; replayed.appended(), 0);
    check!(eq; replayed.receipt(), first.receipt());
    check!(eq; store.receipt(prepared.operation())?, Some(first.receipt().clone()));
    check!(eq; protected_state(&store)?, before_replay);
    exact_root(
        &store.snapshot(),
        held.root(),
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1.phase(),
        reference.as_str(),
        &payload,
    )?;
    drop(snapshot);
    let target = provider_packet(RESULTS)?;
    let target_reference = target.reference()?;
    let target_payload = serde_json::to_value(&target)?;
    let target_recording = Recording::new();
    let before_target = protected_state(&store)?;
    let ProviderTargetIntakeDispositionV1::ProjectedUnaccepted { candidate } =
        record_provider_target_intake(&store, &target_recording, &target, school_budget())?
    else {
        return Err("genuine installed school did not resolve provider target".into());
    };
    check!(eq; protected_state(&store)?, before_target);
    let target_prepared = target_recording
        .drain()
        .prepare("genuine-school-provider-target")?;
    let target_applied = target_prepared.apply(&store)?;
    check!(eq; target_applied.appended(), 4);
    no_public_birth(&store)?;
    drop(store);
    let store = reopen(&directory)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let held = read_recorded_provider_school(&snapshot, &reference, &mut grant)?
        .ok_or("school on combined reopened snapshot")?;
    let target_held = read_recorded_provider_target(&snapshot, &target_reference, &mut grant)?
        .ok_or("provider target after genuine school")?;
    exact_root(
        &snapshot,
        target_held.root(),
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        target_reference.as_str(),
        &target_payload,
    )?;
    check!(eq; held.checked().school(), &school);
    let athletes = snapshot.scan::<CanonicalAthlete>(Table::Athletes)?;
    let [athlete] = athletes.as_slice() else {
        return Err("exactly one projected candidate required".into());
    };
    check!(eq; athlete.id, candidate);
    check!(eq; athlete.school, school.id);
    check!(eq; athlete.canonical_name, "Zacchaeus Brocks");
    check!(eq; athlete.grad_year.get(), 2026);
    check!(eq; athlete.gender, Gender::Boys);
    check!(eq; athlete.identity_in(&SourceNamespace::MilesplitAthlete)
        .ok_or("provider identity")?.id, "7223777");
    observations::verify(&snapshot, target_reference.as_str())?;
    let before_replay = protected_state(&store)?;
    check!(eq; target_prepared.apply(&store)?.receipt(), target_applied.receipt());
    check!(eq; protected_state(&store)?, before_replay);
    check!(eq; store.receipt(target_prepared.operation())?, Some(target_applied.receipt().clone()));
    exact_root(
        &store.snapshot(),
        held.root(),
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1.phase(),
        reference.as_str(),
        &payload,
    )?;
    exact_root(
        &store.snapshot(),
        target_held.root(),
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        target_reference.as_str(),
        &target_payload,
    )?;
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    no_public_birth(&store)?;
    Ok(())
}
