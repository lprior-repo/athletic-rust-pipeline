use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn reference(number: u8) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{number:064x}"))?,
        bytes: 1,
    })
}

fn materials(numbers: &[u8]) -> TestResult<Vec<Material>> {
    numbers
        .iter()
        .map(|number| {
            Ok(Material::Inline {
                reference: reference(*number)?,
                data: "eA==".into(),
            })
        })
        .collect()
}

#[test]
fn sorted_material_lookup_matches_every_key_and_exact_midpoint_count() -> TestResult {
    let rows = materials(&[1, 3, 5, 7, 9, 11, 13])?;
    for (number, comparisons) in [(1, 3), (3, 2), (5, 3), (7, 1), (9, 3), (11, 2), (13, 3)] {
        let expected = reference(number)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let found = introduced_by_digest(&rows, &expected.digest, &mut grant)?
            .ok_or("ordered key absent")?;
        check!(eq; found.reference(), &expected);
        check!(eq; grant.usage().traversal_steps(), comparisons);
        check!(eq; grant.usage().encoded_bytes(), 0);
    }
    Ok(())
}

#[test]
fn missing_keys_and_empty_sets_remain_missing_with_exact_charges() -> TestResult {
    let rows = materials(&[1, 3, 5, 7, 9, 11, 13])?;
    for number in [0, 2, 4, 6, 8, 10, 12, 14] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(introduced_by_digest(&rows, &reference(number)?.digest, &mut grant)?.is_none());
        check!(eq; grant.usage().traversal_steps(), 3);
    }
    let mut grant = AttachmentReplayGrantV1::new();
    check!(introduced_by_digest(&[], &reference(1)?.digest, &mut grant)?.is_none());
    check!(eq; grant.usage().traversal_steps(), 0);
    Ok(())
}

#[test]
fn declared_lookup_keeps_full_digest_and_length_equality() -> TestResult {
    let rows = [reference(1)?, reference(3)?, reference(5)?];
    let expected = reference(3)?;
    let mut wrong_length = expected.clone();
    wrong_length.bytes = 2;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(declared(&rows, &expected, &mut grant)?);
    check!(!declared(&rows, &wrong_length, &mut grant)?);
    check!(eq; grant.usage().traversal_steps(), 3);
    Ok(())
}

#[test]
fn prior_sets_keep_chronological_search_and_complete_missing_comparisons() -> TestResult {
    let sets = [materials(&[1, 5, 9])?, materials(&[3, 7, 11])?];
    let mut grant = AttachmentReplayGrantV1::new();
    check!(eq; find(&sets, &reference(7)?.digest, &mut grant)?.ok_or("second-set key absent")?.reference(), &reference(7)?);
    check!(eq; grant.usage().traversal_steps(), 3);
    check!(find(&sets, &reference(12)?.digest, &mut grant)?.is_none());
    check!(eq; grant.usage().traversal_steps(), 7);
    Ok(())
}

#[test]
fn refused_midpoint_charge_poisons_nonempty_and_empty_retries() -> TestResult {
    let rows = materials(&[1, 3, 5, 7, 9, 11, 13])?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            .checked_sub(1)
            .ok_or("step fixture underflow")?,
    )?;
    check!(matches!(
        introduced_by_digest(&rows, &reference(1)?.digest, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            domain::EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed,
                available
            }
        ))) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq; grant.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    check!(matches!(
        introduced_by_digest(&rows, &reference(1)?.digest, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            domain::EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(matches!(
        introduced_by_digest(&[], &reference(1)?.digest, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            domain::EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    Ok(())
}
