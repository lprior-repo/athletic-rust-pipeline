use super::{fixture::*, *};

#[test]
fn revoke_and_explicit_reattachment_preserve_complete_history() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("attach")?)?;
    let original = records[0].clone();
    let mut revoke = next(&original, "revoke")?;
    revoke.action = AttachmentActionV1::Revoke {
        prior: original.reference(),
    };
    revoke.expected_public_head = AttachmentPublicHeadV1 {
        sequence: 9,
        digest: digest("later changed public head")?,
    };
    append(&mut records, revoke)?;
    check!(eq; replay(&records)?.active_records().count(), 0);
    let mut reattach = next(&records[1], "reattach")?;
    reattach.target.accepted_group_digest = digest("new reviewed group")?;
    let witness = digest("new explicitly reviewed target witness")?;
    add_object(&mut reattach, witness.clone())?;
    reattach.reviewed_context.accepted_target_witness = witness;
    append(&mut records, reattach)?;
    let projection = replay(&records)?;
    check!(eq; projection.records(), records);
    check!(eq;
        projection.active_records().collect::<Vec<_>>(),
        vec![&records[2]]
    );
    check!(eq; projection.inventory(), &inventory(&records)?);
    check!(eq; records[0], original);
    Ok(())
}

#[test]
fn stale_prior_missing_prior_and_second_first_cannot_fork_a_chain() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let second = next(&records[0], "second")?;
    append(&mut records, second)?;
    for action in [
        AttachmentActionV1::Attach {
            prior: AttachmentPredecessorV1::After(records[0].reference()),
        },
        AttachmentActionV1::Attach {
            prior: AttachmentPredecessorV1::First,
        },
        AttachmentActionV1::Revoke {
            prior: AttachmentRecordReferenceV1 {
                operation: text("missing")?,
                digest: digest("missing")?,
            },
        },
    ] {
        let mut fork = records.clone();
        let mut candidate = next(&records[1], "fork")?;
        candidate.action = action;
        append(&mut fork, candidate)?;
        check!(replay(&fork).is_err());
    }
    Ok(())
}

#[test]
fn active_target_changes_duplicate_revocation_and_context_omission_refuse() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let mut changed = next(&records[0], "changed")?;
    changed.target.accepted_group_digest = digest("different active group")?;
    let mut attempted = records.clone();
    append(&mut attempted, changed)?;
    check!(replay(&attempted).is_err());
    let mut revoke = next(&records[0], "revoke")?;
    revoke.action = AttachmentActionV1::Revoke {
        prior: records[0].reference(),
    };
    append(&mut records, revoke)?;
    let mut again = next(&records[1], "again")?;
    again.action = AttachmentActionV1::Revoke {
        prior: records[1].reference(),
    };
    append(&mut records, again)?;
    check!(replay(&records).is_err());
    let mut record = command("omission")?;
    record.reviewed_context.facts = list(vec![])?;
    check!(
        encode_reviewed_reference(&seal(record)?, ReplayLimits::SMALL.total_text_bytes).is_err()
    );
    Ok(())
}

#[test]
fn overlapping_sets_cannot_attach_twice_even_to_same_public_target() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let mut expanded = command("expanded")?;
    let extra = subject("second subject")?;
    add_subject(&mut expanded, extra.clone())?;
    pair_subject(&mut expanded, extra.subject)?;
    append(&mut records, expanded)?;
    check!(matches!(
        replay_reviewed_references(
            &store()?,
            &records,
            &receipts(&records),
            &inventory(&records)?
        ),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "overlapping active subject"
        ))
    ));
    Ok(())
}

#[test]
fn foreign_origin_and_inventory_or_receipt_gaps_refuse() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let foreign = StoreIdentity::parse(&"2".repeat(64))?;
    check!(replay_reviewed_references(
        &foreign,
        &records,
        &receipts(&records),
        &inventory(&records)?
    )
    .is_err());
    check!(replay_reviewed_references(&store()?, &records, &[], &inventory(&records)?).is_err());
    let mut wrong = receipts(&records);
    wrong[0].record_digest = digest("wrong receipt")?;
    check!(replay_reviewed_references(&store()?, &records, &wrong, &inventory(&records)?).is_err());
    check!(replay_reviewed_references(
        &store()?,
        &records,
        &receipts(&records),
        &AttachmentInventoryV1::empty()?
    )
    .is_err());
    let mut second = next(&records[0], "second")?;
    second.expected_attachment_inventory = AttachmentInventoryV1::empty()?;
    records.push(seal(second)?);
    check!(replay(&records).is_err());
    Ok(())
}

#[test]
fn replacement_cannot_drop_original_audited_context() -> TestResult {
    let mut initial = command("first with extra context")?;
    let retained = AttachmentObjectReferenceV1 {
        digest: digest("contrary context object")?,
        bytes: 1,
    };
    let mut objects = initial.reviewed_context.objects.as_slice().to_vec();
    objects.push(retained);
    objects.sort();
    initial.reviewed_context.objects = list(objects)?;
    let mut records = Vec::new();
    append(&mut records, initial)?;
    let mut replacement = next(&records[0], "replacement omits context")?;
    replacement.reviewed_context = command("base context")?.reviewed_context;
    append(&mut records, replacement)?;
    check!(matches!(
        replay_reviewed_references(
            &store()?,
            &records,
            &receipts(&records),
            &inventory(&records)?
        ),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "prior retained context omitted or changed"
        ))
    ));
    Ok(())
}

#[test]
fn retargeting_requires_explicit_revocation_of_the_previous_overlap() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first target")?)?;
    let mut retarget = command("new target")?;
    retarget.target.public_id = crate::public_identity_history::PublicAthleteId::parse(
        "ath_public_v1_afcea59d6afd88c8f0174ae3261f8fcd425bef80224ccd8a1a9a12981aa2c339",
    )?;
    let mut overlapping = records.clone();
    append(&mut overlapping, retarget.clone())?;
    check!(replay(&overlapping).is_err());
    let mut revoke = next(&records[0], "retire old target")?;
    revoke.action = AttachmentActionV1::Revoke {
        prior: records[0].reference(),
    };
    append(&mut records, revoke)?;
    append(&mut records, retarget)?;
    check!(eq;
        replay(&records)?.active_records().collect::<Vec<_>>(),
        vec![&records[2]]
    );
    Ok(())
}

#[test]
fn active_replacement_cannot_refresh_only_the_accepted_target_witness() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let mut replacement = next(&records[0], "witness refresh")?;
    let witness = digest("different active target witness")?;
    add_object(&mut replacement, witness.clone())?;
    replacement.reviewed_context.accepted_target_witness = witness;
    append(&mut records, replacement)?;
    check!(matches!(
        replay_reviewed_references(
            &store()?,
            &records,
            &receipts(&records),
            &inventory(&records)?
        ),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "active target changed"
        ))
    ));
    Ok(())
}
