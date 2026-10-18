use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_domain::model::StoreIdentity;
use census_store::{
    FrozenPublicIdentityBundleV2, ReplayedResultSchoolCorrespondencesV1, StoreError, StoreSnapshot,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SchoolClosureDescriptor {
    object: String,
    reference: AttachmentObjectReferenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchoolClosure {
    bytes: Vec<u8>,
    reference: AttachmentObjectReferenceV1,
}

pub(crate) fn capture_school(
    snapshot: &StoreSnapshot<'_>,
    identity: &FrozenPublicIdentityBundleV2,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<SchoolClosure> {
    grant.charge_steps(1).map_err(domain)?;
    let attachments = snapshot.read_current_evidence_subject_attachment_references_v1(grant)?;
    let current = snapshot.read_current_result_school_correspondences_v1(&attachments, grant)?;
    let proof = current.freeze_closure(identity, grant)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(proof.bytes().len())
        .map_err(|_| invalid("school closure allocation refused"))?;
    bytes.extend_from_slice(proof.bytes());
    Ok(SchoolClosure {
        bytes,
        reference: proof.reference().clone(),
    })
}

pub(crate) fn load_school(
    descriptor: &SchoolClosureDescriptor,
    directory: &Path,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<SchoolClosure> {
    grant.charge_steps(1).map_err(domain)?;
    let bytes = super::objects::load_bytes(directory, &descriptor.object, 4)?;
    if bytes.is_empty() {
        return Err(invalid("school closure bytes absent").into());
    }
    let length =
        u64::try_from(bytes.len()).map_err(|_| invalid("school closure length overflow"))?;
    if length != descriptor.reference.bytes {
        return Err(invalid("school closure length differs").into());
    }
    if super::objects::hash(&bytes) != descriptor.reference.digest.as_str() {
        return Err(invalid("school closure digest differs").into());
    }
    Ok(SchoolClosure {
        bytes,
        reference: descriptor.reference.clone(),
    })
}

impl SchoolClosure {
    pub(crate) fn replay<'a>(
        &'a self,
        store: &str,
        sequence: u64,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<ReplayedResultSchoolCorrespondencesV1<'a>> {
        grant.charge_steps(1).map_err(domain)?;
        if self.bytes.is_empty() {
            return Err(invalid("school closure bytes absent").into());
        }
        let identity =
            StoreIdentity::parse(store).map_err(|_| invalid("school replay store differs"))?;
        Ok(census_store::replay_result_school_source_proof_v1(
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
    ) -> ReportResult<SchoolClosureDescriptor> {
        grant.charge_steps(1).map_err(domain)?;
        if self.bytes.is_empty() {
            return Err(invalid("school closure bytes absent").into());
        }
        let length = u64::try_from(self.bytes.len())
            .map_err(|_| invalid("school closure length overflow"))?;
        if length != self.reference.bytes {
            return Err(invalid("school closure length differs").into());
        }
        let raw: &serde_json::value::RawValue = serde_json::from_slice(&self.bytes)
            .map_err(|_| invalid("school closure bytes differ"))?;
        Ok(SchoolClosureDescriptor {
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
