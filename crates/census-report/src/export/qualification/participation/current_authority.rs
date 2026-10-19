use census_domain::evidence_subject_attachment::{AttachmentInventoryV1, AttachmentReplayGrantV1};
use census_domain::result_calendar_correspondence::ResultCalendarInventoryV1;
use census_domain::result_school_correspondence::ResultSchoolInventoryV1;
use census_store::{
    AcceptedPublicIdentitySnapshot, CurrentEvidenceSubjectAttachmentReferencesV1,
    FrozenCurrentEvidenceSubjectAttachmentReferencesV1, ReplayedResultSchoolCorrespondencesV1,
    StoreError, StoreResult, StoreSnapshot,
};

#[derive(Debug)]
pub(crate) struct DocumentaryAuthority {
    attachments: AttachmentInventoryV1,
    school: Option<ResultSchoolInventoryV1>,
    calendar: Option<ResultCalendarInventoryV1>,
}

impl DocumentaryAuthority {
    pub(crate) fn from_current_attachments(
        current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    ) -> Self {
        let attachments = current.inventory().clone();
        Self {
            attachments,
            school: None,
            calendar: None,
        }
    }

    pub(crate) fn from_frozen_attachments(
        frozen: &FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'_>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        let attachments = frozen.checked_inventory(grant)?.clone();
        Ok(Self {
            attachments,
            school: None,
            calendar: None,
        })
    }

    pub(crate) fn from_school(
        source: &ReplayedResultSchoolCorrespondencesV1<'_>,
        calendar: Option<&census_store::ReplayedResultCalendarCorrespondencesV1<'_>>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        let attachments = source.attachments(grant)?.inventory().clone();
        let calendar = calendar
            .map(|calendar| {
                calendar.require_original_grant(grant)?;
                if calendar.store_identity() != source.store_identity()
                    || calendar.snapshot_sequence() != source.snapshot_sequence()
                    || calendar.attachments(grant)?.inventory() != &attachments
                {
                    return Err(StoreError::Refused {
                        detail: "calendar and school authority bindings differ".into(),
                    });
                }
                Ok(calendar.inventory().clone())
            })
            .transpose()?;
        Ok(Self {
            attachments,
            school: Some(source.inventory().clone()),
            calendar,
        })
    }

    pub(crate) fn require_current(
        &self,
        snapshot: &StoreSnapshot<'_>,
        accepted: &AcceptedPublicIdentitySnapshot,
    ) -> StoreResult<()> {
        let mut grant = AttachmentReplayGrantV1::new();
        let current_attachments = snapshot
            .read_current_evidence_subject_attachment_references_with_accepted_v1(
                accepted, &mut grant,
            )?;
        if current_attachments.inventory() != &self.attachments {
            return Err(StoreError::Refused {
                detail: "current documentary attachment inventory differs".into(),
            });
        }
        self.require_current_school(snapshot, &current_attachments, &mut grant)?;
        self.require_current_calendar(snapshot, &current_attachments, &mut grant)
    }

    fn require_current_school<'s>(
        &self,
        snapshot: &StoreSnapshot<'s>,
        attachments: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, 's>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        if snapshot.result_school_enrollment_proof_v1(grant)?.is_some() {
            let current =
                snapshot.read_current_result_school_correspondences_v1(attachments, grant)?;
            if self.school.as_ref() != Some(current.inventory()) {
                return Err(StoreError::Refused {
                    detail: "current school inventory differs".into(),
                });
            }
        } else if self.school.is_some() {
            return Err(StoreError::Refused {
                detail: "authority has school inventory but current store has none".into(),
            });
        }
        Ok(())
    }

    fn require_current_calendar<'s>(
        &self,
        snapshot: &StoreSnapshot<'s>,
        attachments: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, 's>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        if snapshot
            .result_calendar_enrollment_proof_v1(grant)?
            .is_some()
        {
            let current =
                snapshot.read_current_result_calendar_correspondences_v1(attachments, grant)?;
            if self.calendar.as_ref() != Some(current.inventory()) {
                return Err(StoreError::Refused {
                    detail: "current calendar inventory differs".into(),
                });
            }
        } else if self.calendar.is_some() {
            return Err(StoreError::Refused {
                detail: "authority has calendar inventory but current store has none".into(),
            });
        }
        Ok(())
    }
}
