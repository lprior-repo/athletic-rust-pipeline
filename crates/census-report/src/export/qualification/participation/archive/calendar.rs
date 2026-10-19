use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_domain::model::StoreIdentity;
use census_store::{
    FrozenPublicIdentityBundleV2, ReplayedResultCalendarCorrespondencesV1, StoreError,
    StoreSnapshot,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CalendarClosureDescriptor {
    object: String,
    reference: AttachmentObjectReferenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CalendarClosure {
    bytes: Vec<u8>,
    reference: AttachmentObjectReferenceV1,
}

pub(crate) fn capture_calendar(
    snapshot: &StoreSnapshot<'_>,
    identity: &FrozenPublicIdentityBundleV2,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<CalendarClosure> {
    grant.charge_steps(1).map_err(domain)?;
    let attachments = snapshot.read_current_evidence_subject_attachment_references_v1(grant)?;
    let current = snapshot.read_current_result_calendar_correspondences_v1(&attachments, grant)?;
    let (reference, bytes) = current.freeze_closure(identity, grant)?.into_parts();
    Ok(CalendarClosure { bytes, reference })
}

pub(crate) fn load_calendar(
    descriptor: &CalendarClosureDescriptor,
    directory: &Path,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<CalendarClosure> {
    grant.charge_steps(1).map_err(domain)?;
    let limit = census_store::MAX_RESULT_CALENDAR_SOURCE_PROOF_BYTES;
    let reference_limit =
        u64::try_from(limit).map_err(|_| invalid("calendar closure reference limit overflow"))?;
    if descriptor.reference.bytes == 0 || descriptor.reference.bytes > reference_limit {
        return Err(invalid("calendar closure reference size differs").into());
    }
    let bytes = super::objects::load_bounded_bytes(
        directory,
        &descriptor.object,
        4,
        descriptor.reference.bytes,
        limit,
    )?;
    if bytes.is_empty() {
        return Err(invalid("calendar closure bytes absent").into());
    }
    if super::objects::hash(&bytes) != descriptor.reference.digest.as_str() {
        return Err(invalid("calendar closure digest differs").into());
    }
    Ok(CalendarClosure {
        bytes,
        reference: descriptor.reference.clone(),
    })
}

impl CalendarClosure {
    pub(crate) fn replay<'a>(
        &'a self,
        store: &str,
        sequence: u64,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<ReplayedResultCalendarCorrespondencesV1<'a>> {
        grant.charge_steps(1).map_err(domain)?;
        if self.bytes.is_empty() {
            return Err(invalid("calendar closure bytes absent").into());
        }
        let identity =
            StoreIdentity::parse(store).map_err(|_| invalid("calendar replay store differs"))?;
        Ok(census_store::replay_result_calendar_source_proof_v1(
            &self.bytes,
            &self.reference,
            &identity,
            sequence,
            grant,
        )?)
    }

    pub(crate) fn install(
        &self,
        directory: &Path,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<CalendarClosureDescriptor> {
        grant.charge_steps(1).map_err(domain)?;
        if self.bytes.is_empty() {
            return Err(invalid("calendar closure bytes absent").into());
        }
        let length = u64::try_from(self.bytes.len())
            .map_err(|_| invalid("calendar closure length overflow"))?;
        if length != self.reference.bytes {
            return Err(invalid("calendar closure length differs").into());
        }
        let raw: &serde_json::value::RawValue = serde_json::from_slice(&self.bytes)
            .map_err(|_| invalid("calendar closure bytes differ"))?;
        Ok(CalendarClosureDescriptor {
            object: super::objects::install(&raw, directory, 4)?,
            reference: self.reference.clone(),
        })
    }
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

fn domain(error: EvidenceSubjectAttachmentError) -> StoreError {
    census_store::EvidenceSubjectAttachmentStoreError::from(error).into()
}
