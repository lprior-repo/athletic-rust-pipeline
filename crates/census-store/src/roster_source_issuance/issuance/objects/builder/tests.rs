use super::*;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn exact_aggregate_and_one_more_refuse_before_object_mutation() -> TestResult {
    let builder = Builder::new();
    check!(eq; builder.admit_bytes(MAX_ROSTER_CLOSURE_BYTES)?, MAX_ROSTER_CLOSURE_BYTES);
    check!(builder.admit_bytes(MAX_ROSTER_CLOSURE_BYTES + 1).is_err());
    check!(eq; builder.len(), 0);
    check!(eq; builder.total_bytes, 0);
    Ok(())
}

#[test]
fn checked_sum_refuses_overflow() -> TestResult {
    let mut builder = Builder::new();
    builder.retain("first".into(), vec![1], 1)?;
    check!(builder.admit_bytes(usize::MAX).is_err());
    check!(eq; builder.total_bytes, 1);
    check!(eq; builder.len(), 1);
    Ok(())
}

#[test]
fn real_inserts_duplicate_charges_and_owned_transfer_match_full_map_sum() -> TestResult {
    let mut builder = Builder::new();
    let mut grant = census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new();
    let mut charged = 0_usize;
    for value in 1_u8..=100 {
        let bytes = vec![value; usize::from(value)];
        let first = super::super::insert(&mut builder, &bytes, &mut grant)?;
        let repeated = super::super::insert_owned(&mut builder, bytes.clone(), &mut grant)?;
        check!(eq; first, repeated);
        charged = charged
            .checked_add(bytes.len().checked_mul(2).ok_or("charge overflow")?)
            .ok_or("charge overflow")?;
        let actual = builder
            .values()
            .try_fold(0_usize, |sum, bytes| sum.checked_add(bytes.len()))
            .ok_or("map sum overflow")?;
        check!(eq; builder.total_bytes, actual);
        check!(eq; grant.usage().encoded_bytes(), charged);
        check!(eq; grant.usage().traversal_steps(), usize::from(value) * 2);
    }
    let total = builder.total_bytes;
    let objects = builder.into_objects();
    check!(eq; objects.len(), 100);
    check!(eq; objects.values().map(Vec::len).sum::<usize>(), total);
    Ok(())
}

#[test]
fn stale_or_duplicate_admission_does_not_mutate_builder() -> TestResult {
    let mut builder = Builder::new();
    builder.retain("first".into(), vec![1], 1)?;
    check!(builder.retain("second".into(), vec![2], 1).is_err());
    check!(builder.retain("first".into(), vec![2], 2).is_err());
    check!(eq; builder.total_bytes, 1);
    check!(eq; builder.get("first").map(Vec::as_slice), Some([1].as_slice()));
    check!(!builder.contains_key("second"));
    Ok(())
}
