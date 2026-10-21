use super::*;

pub(super) fn accept(
    store: &Store,
    proposal: &ResultSchoolSourceProposalV1,
    source: &ResultSchoolSourceClaimV1,
    index: usize,
) -> TestResult<(ResultSchoolReviewCommandV1, census_store::Receipt)> {
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let mut command = review::command(
        &inspection,
        source,
        &format!("controlled-cross-provider-tfrrs-school-review-{index}"),
        ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::First,
        },
    )?;
    command.rationale = text("Protocol qualification only: controlled TFRRS transport and an explicitly controlled cross-provider Michigan target for the exact original Indiana quote; no genuine school membership, enrollment interval, cohort or eligibility claim")?;
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_correspondence_v1(
        command.clone(),
        ResultSchoolCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_school_correspondence_v1(prepared, &bytes, &digest)? {
        Application::Written(receipt) => Ok((command, receipt)),
        Application::Repeated(_) => Err("controlled new review repeated".into()),
    }
}
