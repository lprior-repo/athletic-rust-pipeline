use super::{domain, subject, RecordedHtmlResultCaptureV1};
use crate::participation_evidence::result_ownership::CheckedRetainedResultDocumentV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_store::{
    ActiveAttachmentSubjectDiscoveryV1, CurrentEvidenceSubjectAttachmentReferencesV1,
    CurrentEvidenceSubjectAttachmentV1, FrozenCurrentEvidenceSubjectAttachmentReferencesV1,
    FrozenCurrentEvidenceSubjectAttachmentV1, FrozenRetainedCaptureSourceV1,
    RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1, StoreResult, StoreSnapshot,
};

#[derive(Clone, Copy)]
pub(in super::super) enum Source<'a, 's> {
    Current(&'a StoreSnapshot<'s>),
    Frozen(&'a FrozenRetainedCaptureSourceV1<'s>),
}

impl Source<'_, '_> {
    pub(in super::super) fn read(
        self,
        schema: RetainedCaptureRootSchemaV1,
        key: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<RetainedCaptureRootReadV1>> {
        match self {
            Self::Current(view) => view.read_retained_capture_root_v1(schema, key, grant),
            Self::Frozen(view) => view.read_retained_capture_root_v1(schema, key, grant),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum View<'a, 'v, 's> {
    Current(&'a CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>),
    Frozen(&'a FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'s>),
}

pub(super) enum Bindings<'a, 's> {
    Current(Vec<CurrentEvidenceSubjectAttachmentV1<'a, 's>>),
    Frozen(Vec<FrozenCurrentEvidenceSubjectAttachmentV1<'a, 's>>),
}

impl<'a, 'v, 's> View<'a, 'v, 's> {
    pub(super) fn source(self) -> Source<'a, 's> {
        match self {
            Self::Current(current) => Source::Current(current.snapshot()),
            Self::Frozen(current) => Source::Frozen(current.source()),
        }
    }

    pub(super) fn discover(
        self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ActiveAttachmentSubjectDiscoveryV1<'a>> {
        match self {
            Self::Current(current) => current.discover_active_result_subjects_v1(grant),
            Self::Frozen(current) => current.discover_active_result_subjects_v1(grant),
        }
    }

    pub(super) fn bindings(
        self,
        recorded: &RecordedHtmlResultCaptureV1,
        document: &CheckedRetainedResultDocumentV1<'_>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Bindings<'a, 's>> {
        match self {
            Self::Current(current) => bind(recorded, document, grant, |expected, grant| {
                let attached = current
                    .bind_exact_subject(expected, grant)?
                    .ok_or_else(missing)?;
                attached.require_original_grant(grant)?;
                Ok(attached)
            })
            .map(Bindings::Current),
            Self::Frozen(current) => bind(recorded, document, grant, |expected, grant| {
                let attached = current
                    .bind_exact_subject(expected, grant)?
                    .ok_or_else(missing)?;
                attached.require_original_grant(grant)?;
                Ok(attached)
            })
            .map(Bindings::Frozen),
        }
    }
}

fn bind<T>(
    recorded: &RecordedHtmlResultCaptureV1,
    document: &CheckedRetainedResultDocumentV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
    mut issue: impl FnMut(
        &census_domain::evidence_subject_attachment::RetainedDocumentSubjectReferenceV1,
        &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<T>,
) -> CrawlResult<Vec<T>> {
    let count = document.selected_count();
    grant.charge_steps(count).map_err(domain)?;
    let mut attachments = Vec::new();
    attachments
        .try_reserve_exact(count)
        .map_err(|_| domain(EvidenceSubjectAttachmentError::Allocation))?;
    for index in 0..count {
        let row = document
            .row(index)
            .ok_or_else(|| super::super::super::invalid("checked attached result row absent"))?;
        let expected = subject::reference(recorded, &row, grant)?;
        attachments.push(issue(&expected, grant)?);
    }
    Ok(attachments)
}

fn missing() -> census_store::StoreError {
    super::super::super::invalid("original result row has no active current attachment")
}
