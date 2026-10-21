use super::*;
use crate::{Application, PublicSchoolIntakeAdmissionV1, PublicSchoolIntakePacketV1};
use crate::{PublicSchoolIntakeRefV1, RetainedCaptureRootSchemaV1};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

mod fixture;
mod refusal;

#[test]
fn controlled_protocol_intake_mints_original_pair_once_and_preserves_exact_metadata() -> TestResult
{
    let f = Fixture::new()?;
    let (owner, directory, roster) = fixture::pair(&f)?;
    let before = Witness::capture(&f.store)?;
    let prepared = f
        .store
        .prepare_public_school_intake(&owner, &directory, &roster)?;
    let packet = prepared.packet().clone();
    let reference = prepared.reference();
    check!(eq; Witness::capture(&f.store)?, before);
    check!(eq; packet.directory().occurrence(), &directory);
    check!(eq; packet.roster().body_reference(), &roster);
    check!(eq; packet.child_team_id(), "10005");
    fixture::compare_originals(&f.store, &packet)?;
    let Application::Written(receipt) = f.store.apply_public_school_intake(prepared)? else {
        return Err("first intake must write".into());
    };
    let after = Witness::capture(&f.store)?;
    check!(eq; after.sequence, before.sequence.checked_add(1).ok_or("intake sequence")?);
    check!(eq; after.meta.len(), before.meta.len() + 1);
    check!(eq; after.receipts.len(), before.receipts.len() + 1);
    check!(eq; after.journal.len(), before.journal.len() + 1);
    check!(eq; after.entities, before.entities);
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    let proof = f
        .store
        .snapshot()
        .read_public_school_intake_admission_v1(&packet, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; PublicSchoolIntakeAdmissionV1::from_retained(
        &packet, proof.mint_bytes(), proof.receipt_bytes())?, proof);
    let repeated = f.store.apply_public_school_intake(
        f.store
            .prepare_public_school_intake(&owner, &directory, &roster)?,
    )?;
    check!(matches!(repeated, Application::Repeated(held) if held == receipt));
    check!(eq; fixture::read_packet(&f.store, &reference)?.encode()?, packet.encode()?);
    check!(eq; Witness::capture(&f.store)?, after);
    Ok(())
}

#[test]
fn controlled_intake_current_and_both_cold_reads_preserve_packet_mint_and_all_rows() -> TestResult {
    let f = Fixture::new()?;
    let (owner, directory, roster) = fixture::pair(&f)?;
    let prepared = f
        .store
        .prepare_public_school_intake(&owner, &directory, &roster)?;
    let packet = prepared.packet().clone();
    let reference = prepared.reference();
    f.store.apply_public_school_intake(prepared)?;
    let proof = f
        .store
        .snapshot()
        .read_public_school_intake_admission_v1(&packet, &mut AcquisitionReadBudget::for_work())?;
    let before = Witness::capture(&f.store)?;
    let (temporary, root) = fixture::close(f)?;
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; fixture::read_packet(&store, &reference)?.encode()?, packet.encode()?);
        check!(eq; store.snapshot().read_public_school_intake_admission_v1(
            &packet, &mut AcquisitionReadBudget::for_work())?, proof);
        fixture::compare_originals(&store, &packet)?;
        check!(eq; Witness::capture(&store)?, before);
        drop(store);
    }
    drop((owner, temporary));
    Ok(())
}

#[test]
fn controlled_large_roster_uses_standard_chunk_format_and_replays_exact_original_bytes(
) -> TestResult {
    let f = Fixture::new()?;
    let body = vec![b'a'; 1_100_000];
    let (owner, directory, roster) = fixture::pair_with_roster(&f, body.clone())?;
    let prepared = f
        .store
        .prepare_public_school_intake(&owner, &directory, &roster)?;
    let packet = prepared.packet().clone();
    let reference = prepared.reference();
    check!(packet.encode()?.len() > crate::MAX_JOURNAL_VALUE_BYTES);
    let before = Witness::capture(&f.store)?;
    f.store.apply_public_school_intake(prepared)?;
    let after = Witness::capture(&f.store)?;
    check!(after.journal.len() > before.journal.len() + 1);
    check!(after.journal.len() <= before.journal.len() + 7);
    check!(eq; fixture::read_packet(&f.store, &reference)?.roster().body(), body);
    let (temporary, root) = fixture::close(f)?;
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; fixture::read_packet(&store, &reference)?.encode()?, packet.encode()?);
        check!(eq; Witness::capture(&store)?, after);
        drop(store);
    }
    drop((owner, temporary));
    Ok(())
}
