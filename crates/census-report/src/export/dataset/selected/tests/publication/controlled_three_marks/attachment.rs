use super::objects::{self, Objects};
use super::source::{self, InstalledSource, RowLocator, TestResult};
use census_crawl::participation_evidence::provider_target::SourceTargetPacketRefV1;
use census_crawl::participation_evidence::recorded::{
    read_recorded_html_result_capture, read_recorded_independent_participation,
    read_recorded_provider_target, RecordedIndependentParticipationV1,
};
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentCorroborationBasisV1, AttachmentCorroborationPairV1,
    AttachmentDigestV1, AttachmentFactDispositionV1, AttachmentListV1, AttachmentPredecessorV1,
    AttachmentReplayGrantV1, AttachmentReviewActorV1, AttachmentSubjectRoleV1, AttachmentTextV1,
    RetainedDocumentSubjectReferenceV1, RetainedEvidenceSubjectAttachmentCommandV1,
    ReviewedAttachmentContextV1, ReviewedAttachmentFactV1, EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1,
};
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, JournalReadBudget, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1,
    RetainedCaptureSlotV1, StoreSnapshot,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

mod independent;
mod review;
mod rows;
mod target;

#[derive(Default)]
struct Rows {
    subjects: Vec<RetainedDocumentSubjectReferenceV1>,
    facts: Vec<(AttachmentDigestV1, AttachmentDigestV1)>,
    name: Option<String>,
}

pub(super) fn apply(
    installed: &InstalledSource,
    target_status: &ParticipationEvidenceRefV1,
) -> TestResult {
    source::require_originals(&installed.store, &installed.source_records)?;
    let inspection = installed.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: installed.public.clone(),
            prior: AttachmentPredecessorV1::First,
        },
    )?;
    if inspection.target().canonical_candidate != installed.candidate {
        return Err("attachment target differs from installed candidate".into());
    }
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let participation =
        read_recorded_independent_participation(&snapshot, target_status, &mut grant)?
            .ok_or("recorded independent participation absent")?;
    let mut objects = Objects::default();
    let independent = independent::material(
        installed,
        &snapshot,
        &participation,
        &inspection,
        &mut objects,
        &mut grant,
    )?;
    let mut rows = Rows::default();
    rows.subjects
        .try_reserve_exact(installed.row_locators.len())?;
    rows.facts.try_reserve_exact(installed.row_locators.len())?;
    for locator in &installed.row_locators {
        rows::push_row(&snapshot, locator, &mut grant, &mut objects, &mut rows)?;
    }
    if rows.subjects.len() != 3
        || rows.name.as_deref() != Some(participation.checked().subject().name())
    {
        return Err("controlled row identity or inventory differs".into());
    }
    review::commit(installed, inspection, &mut objects, rows, independent)
}

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}
fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}
