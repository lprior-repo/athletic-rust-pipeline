use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

#[test]
fn foreign_original_and_forged_original_digest_refuse_without_rows_in_either_store() -> TestResult {
    let f = Fixture::new()?;
    let (owner, directory, roster) = fixture::pair(&f)?;
    let other = Fixture::new()?;
    let (foreign_owner, foreign_directory, foreign_roster) = fixture::pair(&other)?;
    let before = Witness::capture(&f.store)?;
    let foreign_before = Witness::capture(&other.store)?;
    check!(f
        .store
        .prepare_public_school_intake(&owner, &foreign_directory, &roster)
        .is_err());
    check!(f
        .store
        .prepare_public_school_intake(&foreign_owner, &directory, &foreign_roster)
        .is_err());
    let forged = AcquisitionOccurrenceRefV1::new(
        roster.attempt().clone(),
        different_digest(roster.terminal_digest())?,
    );
    check!(f
        .store
        .prepare_public_school_intake(&owner, &directory, &forged)
        .is_err());
    check!(eq; Witness::capture(&f.store)?, before);
    check!(eq; Witness::capture(&other.store)?, foreign_before);
    Ok(())
}

#[test]
fn prepared_intake_rechecks_actual_archive_before_minting_or_any_row_change() -> TestResult {
    let f = Fixture::new()?;
    let (owner, directory, roster) = fixture::pair(&f)?;
    let prepared = f
        .store
        .prepare_public_school_intake(&owner, &directory, &roster)?;
    let original = f
        .store
        .read_retained_acquisition_v1(&roster, &mut AcquisitionReadBudget::for_work())?;
    std::fs::write(
        original.body_path(),
        b"changed after controlled preparation",
    )?;
    let before = Witness::capture(&f.store)?;
    check!(f.store.apply_public_school_intake(prepared).is_err());
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}

#[test]
fn packet_body_and_mint_credit_forgery_refuse_through_independent_pure_checks() -> TestResult {
    let f = Fixture::new()?;
    let (owner, directory, roster) = fixture::pair(&f)?;
    let prepared = f
        .store
        .prepare_public_school_intake(&owner, &directory, &roster)?;
    let packet = prepared.packet().clone();
    f.store.apply_public_school_intake(prepared)?;
    let before = Witness::capture(&f.store)?;
    let proof = f
        .store
        .snapshot()
        .read_public_school_intake_admission_v1(&packet, &mut AcquisitionReadBudget::for_work())?;
    let mut forged = serde_json::to_value(&packet)?;
    let body = forged
        .get_mut("roster")
        .and_then(|value| value.get_mut("body"))
        .ok_or("roster body")?;
    *body = serde_json::Value::String(STANDARD.encode(b"forged"));
    refused(
        PublicSchoolIntakePacketV1::decode(&serde_json::to_vec(&forged)?),
        "public school intake original body differs",
    )?;
    let mut receipt: crate::Receipt = codec::decode(proof.receipt_bytes())?;
    receipt.credited = true;
    check!(PublicSchoolIntakeAdmissionV1::from_retained(
        &packet,
        proof.mint_bytes(),
        &codec::encode(&receipt)?
    )
    .is_err());
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}

#[test]
fn orphan_mint_packet_and_credited_receipt_refuse_both_cold_opens_without_repair() -> TestResult {
    for fault in ["mint", "packet", "receipt", "credit"] {
        let f = Fixture::new()?;
        let (owner, directory, roster) = fixture::pair(&f)?;
        let prepared = f
            .store
            .prepare_public_school_intake(&owner, &directory, &roster)?;
        let reference = prepared.reference();
        f.store.apply_public_school_intake(prepared)?;
        damage(&f.store, &reference, fault)?;
        let (temporary, root) = fixture::close(f)?;
        let before = fixture::raw_rows(&root)?;
        for current in [true, false] {
            check!(cold_open(&root, current).is_err());
            check!(eq; fixture::raw_rows(&root)?, before);
        }
        drop((owner, temporary));
    }
    Ok(())
}

fn damage(store: &Store, reference: &PublicSchoolIntakeRefV1, fault: &str) -> TestResult {
    let operation = format!("public_resource_school_intake_v1:{}", reference.as_str());
    match fault {
        "mint" => {
            store.meta.remove(
                format!(
                    "public_resource_school_intake/v1/mint/{}",
                    reference.as_str()
                )
                .as_bytes(),
            )?;
        }
        "packet" => {
            store.journal.remove(Store::journal_key(
                "public_resource_school_intake_v1",
                reference.as_str(),
            ))?;
        }
        "receipt" => {
            store.receipts.remove(operation.as_bytes())?;
        }
        "credit" => {
            let mut receipt = store.receipt(&operation)?.ok_or("intake receipt")?;
            receipt.credited = true;
            store
                .receipts
                .insert(operation.as_bytes(), codec::encode(&receipt)?)?;
        }
        _ => return Err("unknown intake fault".into()),
    }
    store.flush()?;
    Ok(())
}

#[test]
fn malformed_aliases_and_noncanonical_chunk_keys_refuse_both_cold_opens_without_repair(
) -> TestResult {
    let id = "a".repeat(64);
    let mut extra_chunk = Store::journal_key("public_resource_school_intake_v1", &id);
    extra_chunk.extend_from_slice(&[1, 0, 0, 0, 0, 9]);
    let aliases = [
        b"public_resource_schoolBAD".to_vec(),
        format!("public_resource_school_intake_v1{id}").into_bytes(),
        Store::journal_key("public_resource_school_intake_v1BAD", &id),
        extra_chunk,
    ];
    for key in aliases {
        let f = Fixture::new()?;
        f.declare("2026-10-14T04:00:00Z")?;
        f.store.journal.insert(&key, b"{}")?;
        let (temporary, root) = fixture::close(f)?;
        let before = fixture::raw_rows(&root)?;
        for current in [true, false] {
            check!(cold_open(&root, current).is_err());
            check!(eq; fixture::raw_rows(&root)?, before);
        }
        drop(temporary);
    }
    Ok(())
}

#[test]
fn terminal_receipt_day_cannot_precede_observation_or_claim_unrecorded_time() -> TestResult {
    let f = Fixture::new()?;
    let (owner, directory, roster) = fixture::pair(&f)?;
    let prepared = f
        .store
        .prepare_public_school_intake(&owner, &directory, &roster)?;
    let original = serde_json::to_value(prepared.packet())?;
    let held = Witness::capture(&f.store)?;
    for (day, reason) in [
        (
            "2026-10-13",
            "public school intake terminal precedes response",
        ),
        (
            "2026-10-21T23:59:59Z",
            "public school intake terminal receipt day",
        ),
        ("2026-1-1", "public school intake terminal receipt day"),
    ] {
        let mut changed = original.clone();
        let at = changed
            .get_mut("roster")
            .and_then(|capture| capture.get_mut("terminal_receipt"))
            .and_then(|receipt| receipt.get_mut("at"))
            .ok_or("terminal receipt day")?;
        *at = serde_json::Value::String(day.into());
        refused(
            PublicSchoolIntakePacketV1::decode(&serde_json::to_vec(&changed)?),
            reason,
        )?;
    }
    check!(eq; Witness::capture(&f.store)?, held);
    Ok(())
}
