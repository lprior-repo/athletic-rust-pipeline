use super::{CandidateQuery, ParticipationInputError, ParticipationObservation, ResultOwnership};
use crate::evidence_subject_attachment::AttachmentSubjectRoleV1;
use crate::public_identity_history::ReplayLimits;

pub(super) fn check(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
) -> Result<(), ParticipationInputError> {
    let ResultOwnership::DocumentaryBound {
        performance,
        source_result,
        subject,
        attachment_inventory,
        proof,
        ..
    } = observation.ownership
    else {
        return Ok(());
    };
    let policy =
        super::participation_policy_v4().map_err(|_| ParticipationInputError::UnsupportedPolicy)?;
    if query.binding.participation_rule != &policy {
        return Err(ParticipationInputError::UnsupportedPolicy);
    }
    if performance != observation.performance {
        return Err(ParticipationInputError::ForeignSubject);
    }
    if subject.role != AttachmentSubjectRoleV1::ResultRow
        || source_result.as_str() != subject.interpretation.as_str()
        || proof.interpretation.as_str() != subject.interpretation.as_str()
        || proof.capture.as_str() != subject.capture.as_str()
        || proof.locator.as_str() != subject.locator.as_str()
        || attachment_inventory.version != 1
        || attachment_inventory.records == 0
        || !usize::try_from(attachment_inventory.records)
            .is_ok_and(|records| records <= ReplayLimits::SMALL.records)
    {
        return Err(ParticipationInputError::InvalidDocumentaryOwnership);
    }
    Ok(())
}
