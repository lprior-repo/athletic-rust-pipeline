use super::*;

#[test]
fn historical_v1_terminal_wire_is_exact_and_new_loss_cannot_enter_it() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let attempt = store
        .reserve_acquisition_attempt(registered(&store, &writer, &owner)?)?
        .attempt_id()
        .clone();
    let terminal = facts::ImmutableAttemptResult::new(
        CaptureProtocolVersion::V1,
        attempt.clone(),
        AcquisitionTerminal::NotDispatched {
            failure: AcquisitionFailureV1::SourceStopped,
        },
    )?;
    let historical = serde_json::json!({"version":1,"attempt":attempt,"terminal":{"kind":"not_dispatched","failure":"source_stopped"}});
    check!(eq; serde_json::to_value(&terminal)?, historical);
    let old: AcquisitionTerminalV1 =
        serde_json::from_value(historical.get("terminal").ok_or("terminal")?.clone())?;
    check!(eq; serde_json::to_value(old)?, serde_json::to_value(terminal.terminal())?);
    let target = PreparedRecoveryTarget::new(
        owner.store_identity().clone(),
        work(&owner)?.slot().id()?,
        attempt.clone(),
        AcquisitionDigest::parse(&"a".repeat(64))?,
    )?;
    let decision = MissingPreparedDecisionV1::new(
        target,
        MissingPreparedAssessmentV1::new(
            MissingLocation::ArchiveDirectory,
            MissingMetadataV1::Missing {
                location: MissingLocation::ArchiveDirectory,
            },
        )?,
    );
    let loss = AcquisitionTerminal::PreparedRetentionFailed {
        prepared_digest: decision.prepared_digest().clone(),
        decision,
    };
    check!(matches!(
        facts::ImmutableAttemptResult::new(
            CaptureProtocolVersion::V1,
            attempt.clone(),
            loss.clone()
        ),
        Err(CaptureError::CaptureProtocolUnsupported)
    ));
    check!(serde_json::from_value::<AcquisitionTerminalV1>(serde_json::to_value(&loss)?).is_err());
    let wrong = serde_json::json!({"version":1,"attempt":attempt,"terminal":loss});
    check!(serde_json::from_value::<facts::ImmutableAttemptResult>(wrong).is_err());
    Ok(())
}

#[test]
fn qualification_version_gate_refuses_other_execution_and_preserves_rows() -> TestResult {
    let (dir, store, _, _) = fixture_for(CaptureProtocolVersion::V2)?;
    let before = rows(&store)?;
    let root = dir.path().join("store");
    drop(store);
    let refused = fixture_for(CaptureProtocolVersion::V1).map(|_| ());
    check!(refused.is_err());
    let error = match refused {
        Err(error) => error,
        Ok(()) => return Err("qualification admitted an unsupported execution version".into()),
    };
    check!(matches!(
        error.downcast_ref::<CaptureError>(),
        Some(CaptureError::CaptureProtocolUnsupported)
    ));
    check!(matches!(
        Store::open_existing(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    check!(matches!(
        Store::migrate(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    let control = Store::capture_qualification_control_existing(&root)?;
    check!(eq; control.show()?.protocol(), CaptureProtocolVersion::V2);
    drop(control);
    let (opened, _) = Store::open_current_qualification_v3(&root)?.into_parts();
    check!(eq; rows(&opened)?, before);
    Ok(())
}

#[test]
fn each_v2_fact_wrapper_refuses_v1_payload_version_before_selector_pin() -> TestResult {
    for suffix in [
        "binding", "reserved", "armed", "response", "prepared", "terminal", "result",
    ] {
        let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let work = work(&owner)?;
        let attempt = occurrence.attempt();
        let key = match suffix {
            "binding" => format!("slot/{}/binding", work.slot().id()?),
            "reserved" | "prepared" | "terminal" => {
                format!("{}/{suffix}", read::attempt_prefix(attempt))
            }
            "result" => format!("work/{}/result", work.id()?),
            _ => format!("{}/dispatch/0/{suffix}", read::attempt_prefix(attempt)),
        };
        let physical = Store::journal_key(owner.protocol().phase(), &key);
        let raw = store.journal.get(&physical)?.ok_or("row")?;
        let mut row: serde_json::Value = serde_json::from_slice(&raw)?;
        row.get_mut("payload")
            .ok_or("payload")?
            .as_object_mut()
            .ok_or("payload object")?
            .insert("version".to_string(), serde_json::json!(1));
        store.journal.insert(&physical, serde_json::to_vec(&row)?)?;
        let before = rows(&store)?;
        check!(matches!(
            store.read_existing_recovery_target(
                &owner,
                &work,
                &mut AcquisitionReadBudget::for_selection()
            ),
            Err(StoreError::Capture(
                CaptureError::CaptureProtocolUnsupported
            ))
        ));
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn sole_family_inventory_refuses_foreign_family_and_foreign_receipt() -> TestResult {
    for keyspace in [crate::JOURNAL, crate::RECEIPTS] {
        let (_dir, store, _, _) = fixture_for(CaptureProtocolVersion::V2)?;
        if keyspace == crate::JOURNAL {
            store.journal.insert(
                Store::journal_key(CaptureProtocolVersion::V1.phase(), "work/foreign/identity"),
                b"{}",
            )?;
        } else {
            store.receipts.insert(
                b"source_acquisition_attempt_v1/work/foreign/attempt/1/terminal",
                b"{}",
            )?;
        }
        check!(matches!(
            super::super::qualification::validate_format(
                store.root(),
                &store.db.snapshot(),
                &store.meta,
                &store.journal,
                &store.receipts,
                &store.store_format()?
            ),
            Err(StoreError::Capture(
                CaptureError::CaptureCapabilityNamespaceUnsupported
            ))
        ));
    }
    Ok(())
}
