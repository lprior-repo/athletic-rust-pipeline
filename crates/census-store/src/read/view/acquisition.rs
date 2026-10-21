use super::StoreSnapshot;
use crate::acquisition::{admission::ReadMeter, qualification, verify_reference};
use crate::{AcquisitionReadBudget, CaptureError, StoreResult, VerifiedRetainedOccurrence};
use census_domain::model::AcquisitionOccurrenceRefV1;

impl StoreSnapshot<'_> {
    pub fn read_public_school_intake_admission_v1(
        &self,
        packet: &crate::PublicSchoolIntakePacketV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<crate::PublicSchoolIntakeAdmissionV1> {
        crate::acquisition::public_resources::intake::read_admission(
            &self.snapshot,
            self.meta,
            self.journal,
            self.receipts,
            packet,
            budget,
        )
    }

    pub fn read_retained_acquisition_v1(
        &self,
        reference: &AcquisitionOccurrenceRefV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<VerifiedRetainedOccurrence> {
        budget.begin_operation();
        let format = crate::format::read_capture_format(&self.snapshot, self.meta, budget)?;
        let declaration = qualification::declaration_admitted(
            &self.snapshot,
            self.meta,
            self.journal,
            &format,
            &mut ReadMeter::Recovery(budget),
        )?
        .ok_or(CaptureError::CaptureCapabilityMissing)?;
        if declaration.store_identity() != reference.attempt().store_identity() {
            return Err(CaptureError::CaptureCapabilityForeignStore.into());
        }
        verify_reference(
            self.root,
            &self.snapshot,
            self.meta,
            self.journal,
            self.receipts,
            reference,
            budget,
            declaration.protocol(),
        )
    }
}
