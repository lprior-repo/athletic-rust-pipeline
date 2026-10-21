use super::*;

pub(super) fn school(
    store: &Store,
    command: ResultSchoolReviewCommandV1,
    expected: &census_store::Receipt,
) -> TestResult {
    let before = state(store)?;
    let prepared = store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::ReplayOnly,
    )?;
    check!(prepared.is_replay());
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(
        matches!(store.apply_result_school_correspondence_v1(prepared, &bytes, &digest)?, Application::Repeated(receipt) if &receipt == expected)
    );
    check!(eq; state(store)?, before);
    Ok(())
}

pub(super) fn calendar(
    store: &Store,
    command: ResultCalendarReviewCommandV1,
    expected: &census_store::Receipt,
) -> TestResult {
    let before = state(store)?;
    let prepared = store.prepare_result_calendar_correspondence_v1(
        command,
        ResultCalendarCorrespondencePreparationV1::ReplayOnly,
    )?;
    check!(prepared.is_replay());
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(
        matches!(store.apply_result_calendar_correspondence_v1(prepared, &bytes, &digest)?, Application::Repeated(receipt) if &receipt == expected)
    );
    check!(eq; state(store)?, before);
    Ok(())
}
