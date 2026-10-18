use super::super::source::TestResult;
use super::objects::{self, Objects};
use census_crawl::participation_evidence::provider_target::CheckedProviderTargetIntakeV1;
use census_crawl::participation_evidence::recorded::RecordedIndependentParticipationV1;
use census_crawl::participation_evidence::SubjectOwnershipV1;
use census_domain::evidence_subject_attachment::AttachmentDigestV1;
use census_store::{
    EvidenceSubjectAttachmentInspectionV1, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1,
    RetainedCaptureSlotV1,
};

pub(super) struct TargetMaterial {
    pub(super) quote: AttachmentDigestV1,
    pub(super) target_subject: AttachmentDigestV1,
    pub(super) lineage: AttachmentDigestV1,
}

pub(super) fn build(
    recorded: &RecordedIndependentParticipationV1,
    provider: &CheckedProviderTargetIntakeV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    body: &[u8],
    objects: &mut Objects,
) -> TestResult<TargetMaterial> {
    let checked = recorded.checked();
    let subject = checked.subject();
    let original = objects::reference(body)?;
    if original.digest.as_str() != subject.body_sha256() {
        return Err("body hash differs from checked original".into());
    }
    match checked.ownership() {
        SubjectOwnershipV1::DocumentOnly(document) if document == subject => {}
        SubjectOwnershipV1::DocumentOnly(_) => {
            return Err("documentary ownership differs from checked subject".into())
        }
        SubjectOwnershipV1::ProviderLink(_) => {
            return Err("attached status requires original documentary ownership".into())
        }
    }
    if subject.name() != provider.profile().name() {
        return Err("document name differs from actual provider profile".into());
    }
    let proof = checked.status_proof();
    if proof.capture() != subject.body_sha256() || proof.locator() != subject.locator() {
        return Err("status proof differs from original documentary subject".into());
    }
    let quote_bytes = owned_quote(proof.locator(), body)?;
    let original = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: recorded.root().clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: original,
    })?;
    let quote = objects.bytes(quote_bytes.to_vec())?;
    let locator = objects.bytes(proof.locator().as_bytes().to_vec())?;
    let target_subject = objects.json(&(
        "reviewed-original-person-specific-target/v1",
        inspection.accepted_target_witness(),
        provider.profile().person(),
        provider.profile().name(),
        subject.document_url(),
        &original,
        &quote,
        &locator,
    ))?;
    let lineage = objects.json(&(
        "reviewed-publisher-family/v1",
        "mhsaa.com",
        subject.document_url(),
        subject.body_sha256(),
    ))?;
    Ok(TargetMaterial {
        quote,
        target_subject,
        lineage,
    })
}

fn owned_quote<'a>(locator: &str, body: &'a [u8]) -> TestResult<&'a [u8]> {
    let span = locator
        .strip_prefix("html_bytes:")
        .ok_or("status locator is not html_bytes")?;
    let (start, end) = span
        .split_once(':')
        .ok_or("status locator endpoints missing")?;
    let start = start.parse::<usize>()?;
    let end = end.parse::<usize>()?;
    Ok(body
        .get(start..end)
        .ok_or("status locator span outside owned body")?)
}
