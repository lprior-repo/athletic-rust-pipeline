use super::*;
use census_store::{ResultSchoolCorrespondenceStoreError, StoreError};

pub(super) fn verify(store: &Store, proposal: &ResultSchoolSourceProposalV1) -> TestResult {
    let before = setup::state(store)?;
    for change_bytes in [true, false] {
        let prepared = refusals::prepare_stale(store, proposal)?;
        let mut approved = prepared.approval_bytes().to_vec();
        let mut digest = prepared.approval_digest().clone();
        if change_bytes {
            let byte = approved
                .first_mut()
                .ok_or("school review approval absent")?;
            *byte ^= 1;
        } else {
            let different = ResultSchoolDigestV1::parse(&"0".repeat(64))?;
            check!(digest != different);
            digest = different;
        }
        check!(matches!(
            store.apply_result_school_correspondence_v1(prepared, &approved, &digest),
            Err(StoreError::ResultSchoolCorrespondence(
                ResultSchoolCorrespondenceStoreError::ApprovalMismatch
            ))
        ));
        check!(eq; setup::state(store)?, before);
    }
    Ok(())
}
