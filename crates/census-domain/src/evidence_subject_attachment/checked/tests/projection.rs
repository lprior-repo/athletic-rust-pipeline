use super::*;

#[test]
fn checked_projection_keeps_shell_receipt_inventory_cost_and_bytes() -> TestResult {
    let dto = seal(command("checked projection cost")?)?;
    let raw = encode_reviewed_reference(&dto, MAXIMUM)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(std::slice::from_ref(&dto), &mut grant)?;
    let before = grant.usage();
    replay(&records, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), before.traversal_steps() + 16);
    let receipt_bytes = serde_json::to_vec(
        &receipts(std::slice::from_ref(&dto))
            .first()
            .ok_or("receipt absent")?,
    )?;
    check!(eq; grant.usage().encoded_bytes(), before.encoded_bytes() + raw.len() + receipt_bytes.len());
    check!(eq; records.records(), &[dto]);
    Ok(())
}

#[test]
fn checked_projection_refuses_receipt_count_and_full_binding_mismatch() -> TestResult {
    let dto = seal(command("checked receipt binding")?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(std::slice::from_ref(&dto), &mut grant)?;
    invalid(
        replay_checked_reviewed_references_with_grant(
            &store()?,
            &records,
            &[],
            &inventory(records.records())?,
            &mut grant,
        ),
        "record and receipt count",
    )?;
    poisoned(&grant)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(std::slice::from_ref(&dto), &mut grant)?;
    let mut wrong = receipts(records.records());
    wrong.first_mut().ok_or("receipt absent")?.record_digest = digest("wrong receipt")?;
    invalid(
        replay_checked_reviewed_references_with_grant(
            &store()?,
            &records,
            &wrong,
            &inventory(records.records())?,
            &mut grant,
        ),
        "record origin or receipt binding",
    )?;
    check!(eq; records.records(), &[dto]);
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn checked_projection_retains_foreign_store_and_complete_inventory_checks() -> TestResult {
    let dto = seal(command("checked origin inventory")?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(std::slice::from_ref(&dto), &mut grant)?;
    invalid(
        replay_checked_reviewed_references_with_grant(
            &StoreIdentity::parse(&"2".repeat(64))?,
            &records,
            &receipts(records.records()),
            &inventory(records.records())?,
            &mut grant,
        ),
        "record origin or receipt binding",
    )?;
    poisoned(&grant)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(std::slice::from_ref(&dto), &mut grant)?;
    check!(matches!(
        replay_checked_reviewed_references_with_grant(
            &store()?,
            &records,
            &receipts(records.records()),
            &AttachmentInventoryV1::empty()?,
            &mut grant,
        ),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "complete attachment inventory"
        ))
    ));
    check!(eq; records.records(), &[dto]);
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn checked_projection_cannot_skip_prefix_or_duplicate_first_chain_checks() -> TestResult {
    let first = seal(command("checked first")?)?;
    let mut second = command("checked second")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(&[first.clone(), seal(second.clone())?], &mut grant)?;
    check!(matches!(
        replay(&records, &mut grant),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "attachment inventory prefix"
        ))
    ));
    poisoned(&grant)?;
    second.expected_attachment_inventory = inventory(std::slice::from_ref(&first))?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(&[first, seal(second)?], &mut grant)?;
    check!(matches!(
        replay(&records, &mut grant),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "second first record"
        ))
    ));
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn checked_projection_cannot_retarget_an_overlapping_active_subject() -> TestResult {
    let first = seal(command("checked active target")?)?;
    let mut second = command("checked different target")?;
    second.expected_attachment_inventory = inventory(std::slice::from_ref(&first))?;
    second.target.public_id = crate::public_identity_history::PublicAthleteId::parse(
        "ath_public_v1_afcea59d6afd88c8f0174ae3261f8fcd425bef80224ccd8a1a9a12981aa2c339",
    )?;
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(&[first, seal(second)?], &mut grant)?;
    check!(matches!(
        replay(&records, &mut grant),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "overlapping active subject"
        ))
    ));
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn checked_projection_keeps_retained_predecessor_context_requirements() -> TestResult {
    let mut initial = command("checked retained context")?;
    let mut objects = initial.reviewed_context.objects.as_slice().to_vec();
    objects.push(AttachmentObjectReferenceV1 {
        digest: digest("retained contrary context")?,
        bytes: 1,
    });
    objects.sort();
    initial.reviewed_context.objects = AttachmentListV1::new(objects)?;
    let first = seal(initial)?;
    let mut second = command("checked omission")?;
    second.expected_attachment_inventory = inventory(std::slice::from_ref(&first))?;
    second.action = AttachmentActionV1::Attach {
        prior: AttachmentPredecessorV1::After(first.reference()),
    };
    let mut grant = AttachmentReplayGrantV1::new();
    let records = collect(&[first, seal(second)?], &mut grant)?;
    check!(matches!(
        replay(&records, &mut grant),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "prior retained context omitted or changed"
        ))
    ));
    poisoned(&grant)?;
    Ok(())
}
