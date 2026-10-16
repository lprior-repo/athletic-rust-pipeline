use super::super::provider_target::{
    CheckedProviderTargetIntakeV1, SourceTargetPacketRefV1, SourceTargetPacketV1,
};
use super::super::result_ownership::{
    read_retained_html_result_rows, CheckedRetainedResultDocumentV1, RetainedResultOwnershipError,
};
use super::super::{
    invalid, IndependentParticipationPacketV1, ParticipationEvidenceRefV1, RetainedHtmlCaptureV1,
    VerifiedIndependentParticipationV1,
};
use crate::milesplit::SourceRowLocator;
use crate::{CrawlError, CrawlResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    EvidenceSubjectAttachmentStoreError, RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1,
    RetainedCaptureRootV1, StoreError, StoreSnapshot,
};
use serde::de::DeserializeOwned;

pub struct RecordedProviderTargetV1 {
    checked: CheckedProviderTargetIntakeV1,
    root: RetainedCaptureRootV1,
}

impl RecordedProviderTargetV1 {
    pub fn checked(&self) -> &CheckedProviderTargetIntakeV1 {
        &self.checked
    }
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
}

pub struct RecordedIndependentParticipationV1 {
    checked: VerifiedIndependentParticipationV1,
    root: RetainedCaptureRootV1,
}

impl RecordedIndependentParticipationV1 {
    pub fn checked(&self) -> &VerifiedIndependentParticipationV1 {
        &self.checked
    }
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
}

pub struct RecordedHtmlResultCaptureV1 {
    capture: RetainedHtmlCaptureV1,
    root: RetainedCaptureRootV1,
}

impl RecordedHtmlResultCaptureV1 {
    pub fn capture(&self) -> &RetainedHtmlCaptureV1 {
        &self.capture
    }
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
    pub fn read(
        &self,
        requested: &[SourceRowLocator],
    ) -> CrawlResult<CheckedRetainedResultDocumentV1<'_>> {
        let document =
            read_retained_html_result_rows(&self.capture, requested).map_err(ownership)?;
        if document.context_reference() != self.root.key {
            return Err(invalid("recorded HTML result context differs from its typed key").into());
        }
        Ok(document)
    }
}

pub fn read_recorded_provider_target(
    snapshot: &StoreSnapshot<'_>,
    reference: &SourceTargetPacketRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedProviderTargetV1>> {
    with_grant(grant, |grant| {
        let Some((packet, held)) = retained::<SourceTargetPacketV1>(
            snapshot,
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
            reference.as_str(),
            grant,
        )?
        else {
            return Ok(None);
        };
        let checked = packet.read()?;
        if checked.reference() != reference {
            return Err(
                invalid("recorded provider target packet differs from its typed key").into(),
            );
        }
        let root = held.root().clone();
        Ok(Some(RecordedProviderTargetV1 { checked, root }))
    })
}

pub fn read_recorded_html_result_capture(
    snapshot: &StoreSnapshot<'_>,
    context_reference: &str,
    requested: &[SourceRowLocator],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedHtmlResultCaptureV1>> {
    with_grant(grant, |grant| {
        let Some((capture, held)) = retained(
            snapshot,
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            context_reference,
            grant,
        )?
        else {
            return Ok(None);
        };
        let document = read_retained_html_result_rows(&capture, requested).map_err(ownership)?;
        if document.context_reference() != context_reference {
            return Err(invalid("recorded HTML result context differs from its typed key").into());
        }
        let root = held.root().clone();
        Ok(Some(RecordedHtmlResultCaptureV1 { capture, root }))
    })
}

pub fn read_recorded_independent_participation(
    snapshot: &StoreSnapshot<'_>,
    reference: &ParticipationEvidenceRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedIndependentParticipationV1>> {
    with_grant(grant, |grant| {
        let Some((packet, held)) = retained::<IndependentParticipationPacketV1>(
            snapshot,
            RetainedCaptureRootSchemaV1::ParticipationPacketV1,
            reference.as_str(),
            grant,
        )?
        else {
            return Ok(None);
        };
        let checked = packet.read(snapshot.store_identity()?, snapshot.sequence())?;
        if checked.reference() != reference {
            return Err(invalid("recorded participation packet differs from its typed key").into());
        }
        let root = held.root().clone();
        Ok(Some(RecordedIndependentParticipationV1 { checked, root }))
    })
}

pub(super) fn with_grant<T>(
    grant: &mut AttachmentReplayGrantV1,
    read: impl FnOnce(&mut AttachmentReplayGrantV1) -> CrawlResult<T>,
) -> CrawlResult<T> {
    grant
        .ensure_available()
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)
        .map_err(StoreError::from)?;
    let result = read(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn retained<T: DeserializeOwned>(
    snapshot: &StoreSnapshot<'_>,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<(T, RetainedCaptureRootReadV1)>> {
    let Some(held) = snapshot.read_retained_capture_root_v1(schema, key, grant)? else {
        return Ok(None);
    };
    let decoded = serde_json::from_str(held.payload().get()).map_err(super::super::json)?;
    Ok(Some((decoded, held)))
}

fn ownership(error: RetainedResultOwnershipError) -> CrawlError {
    match error {
        RetainedResultOwnershipError::Capture(error) => error.into(),
        RetainedResultOwnershipError::Parser(error) => error,
        RetainedResultOwnershipError::Capacity { needed, available } => CrawlError::Resource {
            resource: "recorded HTML result locators",
            requested: needed,
            limit: available,
        },
        RetainedResultOwnershipError::Allocation => CrawlError::Resource {
            resource: "recorded HTML result allocation",
            requested: 1,
            limit: 0,
        },
        RetainedResultOwnershipError::Overflow => StoreError::CounterOverflow.into(),
        RetainedResultOwnershipError::Digest(source) => CrawlError::Canonical {
            table: "recorded HTML result context".into(),
            source,
        },
        RetainedResultOwnershipError::Invalid(detail) => invalid(detail).into(),
    }
}
