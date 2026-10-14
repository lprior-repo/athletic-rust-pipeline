use super::*;

#[test]
fn shared_root_budget_detects_surplus_orphan_and_corrupt_slot_closure() -> TestResult {
    for damage in [
        "surplus",
        "orphan-slot",
        "orphan-work",
        "foreign-slot",
        "wrong-work",
    ] {
        let (_dir, store, writer, owner) = fixture()?;
        let owned = registered(&store, &writer, &owner)?;
        let slot = owned.work().slot().id()?;
        let work_id = owned.work_id().clone();
        let slot_key = format!("slot/{slot}/binding");
        let work_key = format!("work/{work_id}/identity");
        match damage {
            "surplus" => overwrite(
                &store,
                &format!("work/{work_id}/attempt/4/reserved"),
                &"unsupported",
            )?,
            "orphan-slot" => {
                store
                    .journal
                    .remove(Store::journal_key(SOURCE_ACQUISITION_PHASE, &work_key))?;
            }
            "orphan-work" => {
                store
                    .journal
                    .remove(Store::journal_key(SOURCE_ACQUISITION_PHASE, &slot_key))?;
            }
            "foreign-slot" => overwrite(
                &store,
                &slot_key,
                &serde_json::json!({"version":1,"slot":AcquisitionSlotV1::new(StoreIdentity::parse(&"F".repeat(64))?, owner.scope().owner_key(), 2026, UsJurisdiction::Michigan,"123")?,"work_id":work_id}),
            )?,
            "wrong-work" => overwrite(
                &store,
                &slot_key,
                &serde_json::json!({"version":1,"slot":owned.work().slot(),"work_id":"b".repeat(64)}),
            )?,
            _ => return Err("damage".into()),
        }
        let before = rows(&store)?;
        check!(store
            .inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())
            .is_err());
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn corrupt_retryable_work_result_is_rejected_instead_of_suppressing_next_attempt() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let occurrence = retained(&store, &writer, &owner, 429)?;
    overwrite(
        &store,
        &format!("work/{}/result", occurrence.attempt().work_id()),
        &serde_json::json!({"version":1,"occurrence":occurrence}),
    )?;
    check!(registered(&store, &writer, &owner).is_err());
    Ok(())
}

#[test]
fn retained_reference_and_receipt_corruption_fail_with_no_identity_remint() -> TestResult {
    for damage in ["receipt-missing", "receipt-huge", "receipt-digest", "body"] {
        let (_dir, store, writer, owner) = fixture()?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let operation = read::terminal_operation(occurrence.attempt());
        match damage {
            "receipt-missing" => {
                store.receipts.remove(&operation)?;
            }
            "receipt-huge" => {
                store.receipts.insert(&operation, vec![b'x'; 65_537])?;
            }
            "receipt-digest" => {
                store.receipts.insert(&operation, serde_json::to_vec(&serde_json::json!({"operation":operation,"digest":"wrong","at":"2026-10-14","appended":0,"credited":false}))?)?;
            }
            "body" => {
                let verified = store.read_retained_acquisition_v1(
                    &occurrence,
                    &mut AcquisitionReadBudget::for_work(),
                )?;
                std::fs::write(verified.body_path(), b"other content")?;
            }
            _ => return Err("damage".into()),
        }
        let identity = writer.store_identity().clone();
        check!(store
            .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())
            .is_err());
        check!(eq; store.store_identity()?, identity);
    }
    Ok(())
}

#[test]
fn a_reused_budget_never_reuses_backing_verification_across_new_snapshots() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let occurrence = retained(&store, &writer, &owner, 200)?;
    let mut budget = AcquisitionReadBudget::new(84, 8 * 1024 * 1024, 64 * 1024 * 1024)?;
    let verified = store.read_retained_acquisition_v1(&occurrence, &mut budget)?;
    std::fs::write(verified.body_path(), b"altered body!")?;
    check!(matches!(
        store.read_retained_acquisition_v1(&occurrence, &mut budget),
        Err(StoreError::Capture(CaptureError::BackingIntegrity))
    ));
    Ok(())
}

#[test]
fn marker_namespace_version_and_case_preservation_are_checked_before_open_recovery() -> TestResult {
    for damage in [
        "missing",
        "extra",
        "version",
        "foreign",
        "wrong-lifecycle",
        "unsupported-family",
    ] {
        let (dir, store, _writer, _owner) = fixture()?;
        match damage {
            "missing" => {
                store.meta.remove(CAPTURE_QUALIFICATION_KEY)?;
            }
            "extra" => {
                store.meta.insert("capture:v2:qualification", b"{}")?;
            }
            "version" => {
                store
                    .meta
                    .insert(CAPTURE_QUALIFICATION_KEY, br#"{"version":2}"#)?;
            }
            "foreign" => {
                store.meta.insert(
                    CAPTURE_QUALIFICATION_KEY,
                    serialized_bytes(&CaptureQualificationV1::new(StoreIdentity::parse(
                        &"F".repeat(64),
                    )?))?,
                )?;
            }
            "wrong-lifecycle" => {
                store
                    .meta
                    .insert(crate::format::LIFECYCLE_REVISION_KEY, b"2")?;
            }
            "unsupported-family" => {
                store.journal.insert(
                    Store::journal_key("source_acquisition_attempt_v2", "work/unsupported"),
                    b"{}",
                )?;
            }
            _ => return Err("damage".into()),
        }
        store.flush()?;
        drop(store);
        check!(Store::open_capture_qualification_v1(dir.path().join("store")).is_err());
    }
    Ok(())
}
