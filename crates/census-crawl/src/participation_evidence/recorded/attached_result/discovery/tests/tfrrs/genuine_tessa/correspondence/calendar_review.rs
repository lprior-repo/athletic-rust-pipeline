use super::*;

pub(super) fn enroll(store: &Store) -> TestResult {
    let inspection = store.inspect_result_calendar_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_calendar_enrollment_v1(inspection, ResultCalendarEnrollmentReviewV1 {
        operation: ResultCalendarTextV1::parse("genuine-tessa-calendar-correspondence-enrollment")?,
        actor: ResultCalendarReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: ResultCalendarTextV1::parse("Controlled qualification enrollment after exact school review; no inferred venue equivalence or production operator approval")?,
    })?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_result_calendar_enrollment_v1(prepared, &bytes, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

pub(super) fn accept(
    store: &Store,
    proposal: &calendar::ResultCalendarSourceProposalV1,
) -> TestResult<(ResultCalendarReviewCommandV1, census_store::Receipt)> {
    let inspection =
        store.inspect_result_calendar_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = ResultCalendarReviewCommandV1 {
        version: 1, policy: ResultCalendarTextV1::parse(RESULT_CALENDAR_POLICY_V1)?,
        store: inspection.store_identity().clone(), operation: ResultCalendarTextV1::parse("genuine-tessa-exact-roncalli-march11-calendar-correspondence")?,
        input_token: inspection.input_token().clone(), evidence_generation: inspection.evidence_generation(),
        derived_generation: inspection.derived_generation(), accepted_head: inspection.public_head().clone(),
        expected_inventory: inspection.inventory().clone(), source: proposal.claim().clone(),
        action: ResultCalendarActionV1::Accept { prior: ResultCalendarPredecessorV1::First },
        actor: ResultCalendarReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: ResultCalendarTextV1::parse("Controlled test review: exact named Tessa March11 school recap3200 original11:29.42 agrees with native TFRRS row. Preserve school UIndy and provider competition labels; review binds this exact event-day without venue alias, tenure or full-season inference")?,
    };
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_calendar_correspondence_v1(
        command.clone(),
        ResultCalendarCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    check!(!prepared.is_replay());
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_calendar_correspondence_v1(prepared, &bytes, &digest)? {
        Application::Written(receipt) => Ok((command, receipt)),
        Application::Repeated(_) => Err("first genuine Tessa calendar review repeated".into()),
    }
}
