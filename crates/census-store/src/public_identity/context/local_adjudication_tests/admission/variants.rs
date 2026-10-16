use super::*;
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, MembershipAttachmentV1,
};

#[test]
fn graph_preparation_binds_family_and_preserves_the_shared_budget_on_cross_variant_retry(
) -> TestResult {
    let fixture = merged()?;
    let operation = "shared-cap-split";
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let source = prepared.record();
    let prior = source.request.supersedes.first().ok_or("prior merge")?;
    let withheld = LocalMembershipWithholdV1 {
        case_id: source.request.case_id.clone(),
        canonical: source.request.canonical.clone(),
        attachment: MembershipAttachmentV1 {
            sequence: prior.sequence,
            digest: prior.digest.clone(),
            membership_id: "synthetic-cross-family-not-authority".into(),
        },
        expected_withheld_memberships: Vec::new(),
        affected_record_keys: source.request.affected_record_keys.clone(),
        rationale: "Family mismatch must refuse before validating this nonexistent attachment"
            .into(),
    };
    let before = retained_state(&fixture.store)?;
    let counters = admission_counts(&fixture.store)?;
    let result = fixture.writer.prepare_membership_withholding(
        &fixture.store,
        operation,
        withheld,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; admission_counts(&fixture.store)?, counters);
    Ok(())
}
