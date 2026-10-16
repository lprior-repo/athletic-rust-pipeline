use super::*;

#[test]
fn captured_absence_is_distinct_from_unlisted_or_overbroad_namespace() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    let absent = Store::journal_key(PHASE, "absent");
    check!(eq; frozen.journal_size(&absent)?, None);
    check!(frozen.journal_get(&absent)?.is_none());
    check!(raw_text(&frozen, "absent", 1)?.is_none());
    let unknown = Store::journal_key("unlisted", "absent");
    outside(frozen.journal_size(&unknown))?;
    outside(frozen.journal_get(&unknown))?;
    outside(frozen.scan_journal_namespace(
        JournalNamespace::whole_phase("unlisted")?,
        budget(1),
        &mut (),
        |_| Ok(()),
        |_, _, _| Ok(()),
    ))?;
    outside(frozen.visit_receipt_prefix(
        b"evidence_subject",
        budget(1),
        &mut (),
        |_| Ok(()),
        |_, _, _| Ok(()),
    ))?;
    outside(frozen.visit_receipt_prefix(
        b"unrelated",
        budget(1),
        &mut (),
        |_| Ok(()),
        |_, _, _| Ok(()),
    ))?;
    Ok(())
}

#[test]
fn narrower_complete_captured_ranges_can_prove_absence_without_expanding_authority() -> TestResult {
    let fixture = Fixture::new()?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    fixture
        .store
        .receipts
        .insert([PREFIX, b"a"].concat(), b"null")?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    let zero = ReadBudget {
        rows: 0,
        bytes: 0,
        record_bytes: 0,
    };
    let empty = frozen.scan_journal_namespace(
        JournalNamespace::new(PHASE, "absent/")?,
        zero,
        &mut (),
        |_| Err(StoreError::CounterOverflow),
        |_, _, _| Err(StoreError::CounterOverflow),
    )?;
    check!(eq; empty.physical_rows(), 0);
    check!(eq; empty.encoded_bytes(), 0);
    let missing = [PREFIX, b"absent/"].concat();
    let counts = frozen.visit_receipt_prefix(
        &missing,
        zero,
        &mut (),
        |_| Err(StoreError::CounterOverflow),
        |_, _, _| Err(StoreError::CounterOverflow),
    )?;
    check!(eq; counts.key_visits, 0);
    check!(eq; counts.encoded_bytes, 0);
    outside(frozen.scan_journal_namespace(
        JournalNamespace::new("evidence", "subject")?,
        zero,
        &mut (),
        |_| Ok(()),
        |_, _, _| Ok(()),
    ))?;
    Ok(())
}

#[test]
fn bundle_reference_length_and_digest_refusals_poison_the_original_grant() -> TestResult {
    let bytes = b"{}".to_vec();
    let mut expected = reference(&bytes)?;
    expected.bytes = expected
        .bytes
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        Bundle::from_owned_bytes(bytes.clone(), expected, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen attachment archive length differs"
        )))
    ));
    poisoned(&mut grant)?;
    let mut expected = reference(&bytes)?;
    expected.digest = AttachmentDigestV1::parse(&"0".repeat(64))?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        Bundle::from_owned_bytes(bytes, expected, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen attachment archive digest differs"
        )))
    ));
    poisoned(&mut grant)
}

fn poisoned(grant: &mut AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        grant.charge_bytes(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
