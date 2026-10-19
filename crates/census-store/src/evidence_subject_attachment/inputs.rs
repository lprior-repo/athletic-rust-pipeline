use super::{
    encoding::{self, EncodedObject},
    EvidenceSubjectAttachmentStoreError as Error,
};
use crate::public_identity::paged_generation::{
    attachment_capture_binding, require_attachment_capture_inputs, CaptureBindingV2,
};
use crate::{Store, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{AttachmentPublicHeadV1, AttachmentReplayGrantV1};
use census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE;

pub(super) struct Inputs {
    pub(super) capture: EncodedObject,
    pub(super) token: EncodedObject,
    pub(super) binding: CaptureBindingV2,
}

impl Inputs {
    pub(super) fn capture(
        store: &Store,
        view: &StoreSnapshot<'_>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        grant.charge_steps(1).map_err(Error::Domain)?;
        require_format(store)?;
        grant.charge_steps(3).map_err(Error::Domain)?;
        let binding = attachment_capture_binding(store, view)?;
        let capture = encoding::object(&binding, grant)?;
        grant.charge_steps(2).map_err(Error::Domain)?;
        let raw = view.attachment_input_revision_bytes()?;
        let token = encoding::raw_object(&raw, grant)?;
        Ok(Self {
            capture,
            token,
            binding,
        })
    }

    pub(super) fn require_current(
        &self,
        store: &Store,
        view: &StoreSnapshot<'_>,
        evidence: u64,
        derived: u64,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        grant.charge_steps(1).map_err(Error::Domain)?;
        require_format(store)?;
        grant.charge_steps(3).map_err(Error::Domain)?;
        let current = attachment_capture_binding(store, view)?;
        grant.charge_steps(1).map_err(Error::Domain)?;
        require_attachment_capture_inputs(&self.binding, &current, evidence, derived)?;
        grant.charge_steps(2).map_err(Error::Domain)?;
        let token = view.attachment_input_revision_bytes()?;
        grant.charge_steps(1).map_err(Error::Domain)?;
        if token != self.token.bytes() {
            return Err(Error::InputMoved.into());
        }
        Ok(())
    }
}

pub(super) fn require_format(store: &Store) -> StoreResult<()> {
    let format = store.store_format()?;
    if !matches!(
        format.lifecycle_revision,
        Some(
            PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE
                | crate::ROSTER_SOURCE_ISSUANCE_LIFECYCLE
                | crate::RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE
                | crate::RESULT_CALENDAR_CORRESPONDENCE_LIFECYCLE
        )
    ) || format.migration_target.is_some()
    {
        return Err(Error::InputMoved.into());
    }
    Ok(())
}

pub(super) fn require_head(
    view: &StoreSnapshot<'_>,
    expected: &AttachmentPublicHeadV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(2).map_err(Error::Domain)?;
    let head = view.attachment_public_head()?;
    if head.sequence != expected.sequence || head.digest != expected.digest.as_str() {
        return Err(Error::HeadMoved.into());
    }
    Ok(())
}
