use census_domain::evidence_subject_attachment::{AttachmentInventoryV1, AttachmentReplayGrantV1};
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
}

impl DocumentaryAuthority {
    pub(crate) fn from_current_attachments(
        current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    ) -> Self {
        let attachments = current.inventory().clone();
        Self {
            attachments,
            school: None,
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
        })
    }

    pub(crate) fn from_school(
        source: &ReplayedResultSchoolCorrespondencesV1<'_>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        let attachments_inventory = source.attachments(grant)?.inventory().clone();
        let school_inventory = source.inventory().clone();
        Ok(Self {
            attachments: attachments_inventory,
            school: Some(school_inventory),
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
        if snapshot
            .result_school_enrollment_proof_v1(&mut grant)?
            .is_some()
        {
            let current_school = snapshot
                .read_current_result_school_correspondences_v1(&current_attachments, &mut grant)?;
            let actual_school = current_school.inventory();
            match &self.school {
                Some(expected) => {
                    if actual_school != expected {
                        return Err(StoreError::Refused {
                            detail: "current school inventory differs".into(),
                        });
                    }
                }
                None => {
                    return Err(StoreError::Refused {
                        detail: "current school inventory present but authority has none".into(),
                    });
                }
            }
        } else if self.school.is_some() {
            return Err(StoreError::Refused {
                detail: "authority has school inventory but current store has none".into(),
            });
        }
        Ok(())
    }
}
