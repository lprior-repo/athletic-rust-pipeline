use super::*;
use census_crawl::participation_evidence::recorded::read_recorded_independent_participation;

pub(super) struct Material {
    pub subject: RetainedDocumentSubjectReferenceV1,
    pub fact: AttachmentDigestV1,
    pub lineage: AttachmentDigestV1,
    pub target_fact: AttachmentDigestV1,
    pub target_lineage: AttachmentDigestV1,
    pub basis: AttachmentCorroborationBasisV1,
    pub independent_context: AttachmentDigestV1,
}

pub(super) fn subject(
    objects: &mut Objects,
    document: &census_crawl::participation_evidence::DocumentSubjectV1,
) -> TestResult<AttachmentDigestV1> {
    let digest = objects.bytes(
        format!(
            "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
            document.document_namespace(),
            document.native_document_id(),
            document.document_url(),
            document.body_sha256(),
            document.locator()
        )
        .into_bytes(),
    )?;
    check!(eq; digest.as_str(), document.reference());
    Ok(digest)
}

mod result;
mod status;
mod target;

pub(super) use result::read as result;
pub(super) use status::read as status;
