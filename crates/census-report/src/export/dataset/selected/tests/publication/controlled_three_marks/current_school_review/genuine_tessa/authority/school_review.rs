use super::*;

pub(super) fn enroll(store: &Store) -> TestResult {
    let inspection = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(inspection, ResultSchoolEnrollmentReviewV1 {
        operation: ResultSchoolTextV1::parse("genuine-tessa-school-correspondence-enrollment")?,
        actor: ResultSchoolReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: ResultSchoolTextV1::parse("Controlled qualification enrollment for exact original Roncalli school review; not production operator approval")?,
    })?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_result_school_enrollment_v1(prepared, &bytes, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

pub(super) fn accept(
    store: &Store,
    proposal: &school::ResultSchoolSourceProposalV1,
) -> TestResult<(ResultSchoolReviewCommandV1, census_store::Receipt)> {
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = ResultSchoolReviewCommandV1 {
        version: 1, policy: ResultSchoolTextV1::parse(RESULT_SCHOOL_POLICY_V1)?,
        store: inspection.store_identity().clone(), operation: ResultSchoolTextV1::parse("genuine-tessa-exact-roncalli-school-correspondence")?,
        input_token: inspection.input_token().clone(), evidence_generation: inspection.evidence_generation(),
        derived_generation: inspection.derived_generation(), accepted_head: inspection.public_head().clone(),
        expected_inventory: inspection.inventory().clone(), source: proposal.claim().clone(),
        action: ResultSchoolActionV1::Accept { prior: ResultSchoolPredecessorV1::First },
        actor: ResultSchoolReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: ResultSchoolTextV1::parse("Controlled test review: literal Roncalli original TFRRS team span and independently captured native Roncalli school10561 directory; exact March11 only, no tenure interval")?,
    };
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_correspondence_v1(
        command.clone(),
        ResultSchoolCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    check!(!prepared.is_replay());
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_school_correspondence_v1(prepared, &bytes, &digest)? {
        Application::Written(receipt) => Ok((command, receipt)),
        Application::Repeated(_) => Err("first genuine Tessa school review repeated".into()),
    }
}
