use super::*;

#[test]
fn never_bound_empty_collection_replays_with_each_available_grant() -> TestResult {
    let records = CheckedReviewedReferenceRecordsV1::new();
    for _ in 0..2 {
        let mut grant = AttachmentReplayGrantV1::new();
        replay(&records, &mut grant)?;
        check!(eq; grant.usage().traversal_steps(), 3);
        check!(eq; grant.usage().encoded_bytes(), 0);
    }
    check!(records.records().is_empty());
    Ok(())
}

#[test]
fn emptied_bound_collection_preserves_its_original_grant_origin() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let record = draft("emptied collection", &mut grant)?;
    let dto = record.record().clone();
    let mut records = CheckedReviewedReferenceRecordsV1::new();
    records.push_with_grant(record, &mut grant)?;
    let removed = records.pop_with_grant(&mut grant)?;
    check!(eq; removed.record(), &dto);
    check!(records.records().is_empty());
    replay(&records, &mut grant)?;
    let mut fresh = AttachmentReplayGrantV1::new();
    invalid(
        replay(&records, &mut fresh),
        "checked command grant differs",
    )?;
    poisoned(&fresh)?;
    records.push_with_grant(removed, &mut grant)?;
    check!(eq; records.records(), &[dto]);
    Ok(())
}

#[test]
fn foreign_record_push_refuses_before_changing_bound_collection() -> TestResult {
    let dto = seal(command("bound collection")?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let mut records = collect(std::slice::from_ref(&dto), &mut grant)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let record = draft("foreign record", &mut foreign)?;
    invalid(
        records.push_with_grant(record, &mut grant),
        "checked command grant differs",
    )?;
    check!(eq; records.records(), &[dto]);
    poisoned(&grant)?;
    foreign.ensure_available()?;
    Ok(())
}

#[test]
fn matching_record_cannot_rebind_a_foreign_collection_on_push() -> TestResult {
    let dto = seal(command("original collection anchor")?)?;
    let mut original = AttachmentReplayGrantV1::new();
    let mut records = collect(std::slice::from_ref(&dto), &mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let record = draft("matches foreign grant", &mut foreign)?;
    let before = foreign.usage().traversal_steps();
    invalid(
        records.push_with_grant(record, &mut foreign),
        "checked command grant differs",
    )?;
    check!(eq; foreign.usage().traversal_steps(), before + 2);
    check!(eq; records.records(), &[dto]);
    poisoned(&foreign)?;
    original.ensure_available()?;
    Ok(())
}

#[test]
fn wrong_grant_pop_and_remove_leave_the_complete_collection_unchanged() -> TestResult {
    let dto = seal(command("wrong extraction grant")?)?;
    let mut original = AttachmentReplayGrantV1::new();
    let mut records = collect(std::slice::from_ref(&dto), &mut original)?;
    let mut pop_grant = AttachmentReplayGrantV1::new();
    invalid(
        records.pop_with_grant(&mut pop_grant),
        "checked command grant differs",
    )?;
    poisoned(&pop_grant)?;
    let mut remove_grant = AttachmentReplayGrantV1::new();
    invalid(
        records.remove_with_grant(0, &mut remove_grant),
        "checked command grant differs",
    )?;
    poisoned(&remove_grant)?;
    check!(eq; records.records(), &[dto]);
    original.ensure_available()?;
    Ok(())
}

#[test]
fn absent_extraction_refuses_without_issuing_or_removing_an_owner() -> TestResult {
    let mut empty = CheckedReviewedReferenceRecordsV1::new();
    let mut grant = AttachmentReplayGrantV1::new();
    invalid(
        empty.pop_with_grant(&mut grant),
        "checked record extraction absent",
    )?;
    check!(eq; grant.usage().traversal_steps(), 1);
    check!(empty.records().is_empty());
    poisoned(&grant)?;
    let dto = seal(command("out of range extraction")?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let mut records = collect(std::slice::from_ref(&dto), &mut grant)?;
    let before = grant.usage().traversal_steps();
    invalid(
        records.remove_with_grant(usize::MAX, &mut grant),
        "checked record extraction absent",
    )?;
    check!(eq; grant.usage().traversal_steps(), before + 1);
    check!(eq; records.records(), &[dto]);
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn successful_remove_moves_exact_owner_and_preserves_remaining_order() -> TestResult {
    let first = seal(command("first collection slot")?)?;
    let second = seal(command("second collection slot")?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let mut records = collect(&[first.clone(), second.clone()], &mut grant)?;
    let before = grant.usage().traversal_steps();
    let removed = records.remove_with_grant(0, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), before + 2);
    check!(eq; removed.record(), &first);
    check!(eq; records.records(), std::slice::from_ref(&second));
    check!(eq; removed.encode_with_grant(MAXIMUM, &mut grant)?, encode_reviewed_reference(&first, MAXIMUM)?);
    check!(eq; records.into_records(), vec![second]);
    Ok(())
}
