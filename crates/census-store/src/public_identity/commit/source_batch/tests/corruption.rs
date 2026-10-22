use super::*;

#[test]
fn completion_tampering_and_deletion_refuse_qualification_and_replay() -> TestResult {
    for mutation in 0..8 {
        let (_directory, store, writer) = fixture()?;
        append_pair(&store)?;
        commit(&writer, &store, "cohort-tamper")?;
        let request = Request::new(writer.store_identity(), "cohort-tamper", GradYear::CO2027)?;
        let ledger = writer.verified(&store)?;
        let mut receipt = completion(&store, &request)?;
        match mutation {
            0 => receipt.appended = receipt.appended.checked_add(1).ok_or("count overflow")?,
            1 => receipt.appended = 1,
            2 => receipt.at.push_str("-changed"),
            3 => receipt.digest = crate::journal::object_digest(b"different request"),
            4 => receipt.credited = true,
            5 => receipt.operation.push_str("-changed"),
            6 => receipt.appended = 0,
            _ => {}
        }
        if mutation == 7 {
            let mut batch = store.db.batch();
            batch.remove(&store.receipts, &request.operation);
            batch.durability(Some(PersistMode::SyncData)).commit()?;
        } else {
            write_completion(&store, &request.operation, &receipt)?;
        }
        let before = retained_state(&store)?;
        check!(audit_records(&store, &writer, ledger.history.records()).is_err());
        check!(writer.head(&store).is_err());
        check!(commit(&writer, &store, "cohort-tamper").is_err());
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn orphan_completion_with_valid_raw_alias_refuses_without_fact_children() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    append_pair(&store)?;
    commit(&writer, &store, "cohort-valid")?;
    let request = Request::new(writer.store_identity(), "cohort-orphan", GradYear::CO2027)?;
    let head = writer.head(&store)?;
    let capture = graph_operation::begin(
        &writer,
        &store,
        &request.operation,
        &request.digest,
        store.snapshot().revision(),
        &head,
    )?;
    let ledger = writer.verified(&store)?;
    let receipt = crate::Receipt {
        operation: request.operation.clone(),
        digest: request.digest.clone(),
        at: capture.observed_at,
        appended: 1,
        credited: false,
    };
    write_completion(&store, &request.operation, &receipt)?;
    check!(audit_records(&store, &writer, ledger.history.records()).is_err());
    check!(writer.head(&store).is_err());
    Ok(())
}

#[test]
fn missing_or_changed_normal_child_receipt_keeps_existing_fact_proof_refusal() -> TestResult {
    for remove in [false, true] {
        let (_directory, store, writer) = fixture()?;
        append_pair(&store)?;
        commit(&writer, &store, "cohort-child-receipt")?;
        let ledger = writer.verified(&store)?;
        let record = ledger
            .history
            .records()
            .first()
            .ok_or("first child record")?;
        let key = witness::key(record.sequence);
        let raw = codec::load(&store.db.snapshot(), &store.journal, &key, 65_536)?
            .ok_or("child witness")?;
        let held = HeldGraphWitness::decode(&key, &raw)?;
        let key = witness::receipt_key(held.operation());
        let mut batch = store.db.batch();
        if remove {
            batch.remove(&store.receipts, &key);
        } else {
            let raw = codec::load(&store.db.snapshot(), &store.receipts, &key, 65_536)?
                .ok_or("child receipt")?;
            let mut receipt: crate::Receipt = codec::decode(&key, &raw)?;
            receipt.digest = crate::journal::object_digest(b"changed child witness");
            batch.insert(&store.receipts, &key, codec::encode(&receipt, 65_536)?);
        }
        batch.durability(Some(PersistMode::SyncData)).commit()?;
        check!(writer.head(&store).is_err());
    }
    Ok(())
}

#[test]
fn completion_requires_original_raw_reservation_owner() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    append_pair(&store)?;
    commit(&writer, &store, "cohort-alias")?;
    let request = Request::new(writer.store_identity(), "cohort-alias", GradYear::CO2027)?;
    let ledger = writer.verified(&store)?;
    let operation = graph_operation::raw_operation(writer.store_identity(), &request.operation)?;
    let key = crate::public_identity::local_adjudication::admission::key(operation.as_str());
    let mut batch = store.db.batch();
    batch.remove(&store.journal, &key);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    check!(audit_records(&store, &writer, ledger.history.records()).is_err());
    check!(writer.head(&store).is_err());
    Ok(())
}
