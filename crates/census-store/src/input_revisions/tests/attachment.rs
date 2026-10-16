use super::*;

#[test]
fn attachment_token_encoding_preserves_both_baselines_and_all_six_counters() -> TestResult {
    let token = PublicIdentityInputRevisionTokenV2 {
        baseline_evidence_generation: 13,
        baseline_derived_generation: 17,
        counters: [19, 23, 29, 31, 37, u64::MAX],
    };
    let raw = codec::encode(token)?;
    check!(eq; raw, br#"{"version":2,"baseline_evidence_generation":13,"baseline_derived_generation":17,"counters":[19,23,29,31,37,18446744073709551615]}"#.to_vec());
    Ok(())
}

#[test]
fn attachment_token_bytes_require_enabled_and_remain_pinned() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let disabled = store.snapshot();
    check!(disabled.attachment_input_revision_bytes().is_err());
    let baseline = enroll(&store)?;
    let held = store.snapshot();
    check!(eq; held.attachment_input_revision_bytes()?, codec::encode(baseline)?);
    mutate(&store, Table::Athletes, "later")?;
    check!(eq; held.attachment_input_revision_bytes()?, codec::encode(baseline)?);
    check!(ne; store.snapshot().attachment_input_revision_bytes()?, held.attachment_input_revision_bytes()?);
    check!(disabled.attachment_input_revision_bytes().is_err());
    Ok(())
}
