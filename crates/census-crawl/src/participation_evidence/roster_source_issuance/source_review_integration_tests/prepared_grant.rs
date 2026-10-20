use super::*;

#[test]
fn prepared_review_refuses_exhausted_grant_before_and_during_apply() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let operation = "exhausted-grant-safety";
    let mut prepared = prepare(&store, operation, None)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let before = store.snapshot().revision();
    {
        let (_, grant) = prepared.source_parts()?;
        check!(grant.charge_bytes(usize::MAX).is_err());
    }
    check!(prepared.proposal_claim().is_err());
    check!(store
        .apply_roster_source_review_v1(prepared, &approved, &digest)
        .is_err());
    check!(store
        .replay_roster_source_review_v1(operation, &digest)
        .is_err());
    check!(eq; store.snapshot().revision().evidence_generation(), before.evidence_generation());
    check!(eq; store.snapshot().revision().derived_generation(), before.derived_generation());
    Ok(())
}
