use super::{with_grant, CheckedProviderSchoolPacketV1, RetainedProviderSchoolPacketV1};
use crate::participation_evidence::invalid;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    EvidenceSubjectAttachmentStoreError, RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1,
    StoreError,
};

pub(crate) fn decode(
    schema: RetainedCaptureRootSchemaV1,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RetainedProviderSchoolPacketV1> {
    with_grant(grant, |grant| {
        if schema == RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1 {
            grant
                .charge_bytes(
                    raw.len()
                        .checked_mul(3)
                        .ok_or(StoreError::CounterOverflow)?,
                )
                .map_err(EvidenceSubjectAttachmentStoreError::Domain)
                .map_err(StoreError::from)?;
            grant
                .charge_steps(64)
                .map_err(EvidenceSubjectAttachmentStoreError::Domain)
                .map_err(StoreError::from)?;
        }
        Ok(RetainedProviderSchoolPacketV1::from_root(schema, raw)?)
    })
}

pub(crate) fn original(
    held: &RetainedCaptureRootReadV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    with_grant(grant, |grant| {
        let packet = decode(held.root().schema, held.payload().get().as_bytes(), grant)?;
        let checked = match packet {
            RetainedProviderSchoolPacketV1::Raw(packet) => packet.read_semantic_contract_v1()?,
            RetainedProviderSchoolPacketV1::Owned(packet) => packet.read()?,
        };
        if checked.reference().as_str() != held.root().key {
            return Err(
                invalid("recorded provider school packet differs from its typed key").into(),
            );
        }
        Ok(checked)
    })
}
