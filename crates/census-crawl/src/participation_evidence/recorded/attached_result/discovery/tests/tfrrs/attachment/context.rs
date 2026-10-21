use super::*;
use crate::participation_evidence::result_ownership::tfrrs::CheckedRetainedTfrrsListV1;
use census_store::RetainedCaptureRootV1;

pub(super) struct Review {
    pub subject: RetainedDocumentSubjectReferenceV1,
    pub facts: Vec<ReviewedAttachmentFactV1>,
    pub pair: AttachmentCorroborationPairV1,
    pub scope: AttachmentDigestV1,
    pub target: AttachmentDigestV1,
}

pub(super) fn build(
    objects: &mut Objects,
    checked: &CheckedRetainedTfrrsListV1<'_>,
    index: usize,
    root: &RetainedCaptureRootV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    case: Case,
) -> TestResult<Review> {
    let row = checked.row(index).ok_or("genuine native source row")?;
    let source = original::read(objects, checked, &row, root, case)?;
    let native_anchor = checked
        .original_document()
        .get(row.athlete_span())
        .ok_or("original native athlete anchor")?;
    check!(native_anchor.contains(row.athlete_id()));
    check!(native_anchor.contains(row.athlete_name()));
    let target_fact = objects.bytes(native_anchor.as_bytes().to_vec())?;
    let target = objects.json(&(
        "controlled-target-explicit-native-reference/v1",
        inspection.target(),
        inspection.accepted_target_witness(),
        row.athlete_id(),
        row.athlete_path(),
        row.athlete_name(),
    ))?;
    let mut facts = vec![
        ReviewedAttachmentFactV1 {
            fact: source.fact.clone(), subject: source.subject.subject.clone(),
            publisher_lineage: source.lineage.clone(),
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Whole exact original row retained; no source fact inferred from test target")?,
        },
        ReviewedAttachmentFactV1 {
            fact: target_fact.clone(), subject: target.clone(), publisher_lineage: source.lineage,
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Exact original native athlete anchor matches this separate source-bound target; same publisher, no independent-source claim")?,
        },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let pair = AttachmentCorroborationPairV1 {
        subject_fact: source.fact, accepted_target_fact: target_fact,
        basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
        explanation: text("Controlled test review of exact original TFRRS native athlete link to its matching source-bound target; no recruiting or school authority granted")?,
    };
    let scope = objects.json(&(
        "controlled-tfrrs-attachment-complete-scope/v1",
        "Genuine original body; controlled transport and test reviewer decision. No membership, cohort, independent corroboration or eligibility asserted.",
        format!("{case:?}"), root, &source.subject, &facts, &pair, &target,
        inspection.target(), inspection.accepted_target_witness(),
    ))?;
    Ok(Review {
        subject: source.subject,
        facts,
        pair,
        scope,
        target,
    })
}
