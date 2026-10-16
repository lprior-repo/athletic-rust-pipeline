use super::types::{Diagnostic, Member};
use super::{domain, grouping, invalid, reserve, AttachmentHtmlResultDiscoveryGapV1 as Gap};
use crate::participation_evidence::result_ownership::discovery::OriginalResultSpan;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    ActiveResultSubjectDiscoveryV1, ActiveResultSubjectV1, DiscoveredAttachmentCaptureV1,
    DiscoveredAttachmentLocatorV1, RetainedCaptureLeafKindV1, RetainedCaptureRootSchemaV1,
    RetainedCaptureSlotV1,
};

pub(super) fn all(
    metadata: &ActiveResultSubjectDiscoveryV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<(Vec<Member>, Vec<Diagnostic>)> {
    let count = metadata.subjects().len();
    let mut members = reserve(count, grant)?;
    let mut diagnostics = reserve(count, grant)?;
    for (index, source) in metadata.subjects().iter().enumerate() {
        grant.charge_steps(1).map_err(domain)?;
        match classify(source, grant)? {
            Ok(span) => grouping::insert(metadata, &mut members, Member { index, span }, grant)?,
            Err(reason) => diagnostics.push(Diagnostic { index, reason }),
        }
    }
    Ok((members, diagnostics))
}

fn classify(
    source: &ActiveResultSubjectV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Result<OriginalResultSpan, Gap>> {
    let DiscoveredAttachmentCaptureV1::Retained(leaf) = source.capture() else {
        return Ok(Err(Gap::UnsupportedCaptureMaterial));
    };
    if leaf.root.schema != RetainedCaptureRootSchemaV1::HtmlResultCaptureV1 {
        return Ok(Err(Gap::UnsupportedCaptureSchema));
    }
    if leaf.slot != RetainedCaptureSlotV1::SelfCapture {
        return Ok(Err(Gap::UnsupportedCaptureSlot));
    }
    if leaf.kind != RetainedCaptureLeafKindV1::CaptureJson {
        return Ok(Err(Gap::UnsupportedCaptureKind));
    }
    grant.charge_steps(1).map_err(domain)?;
    if leaf.version != 1
        || leaf.object != leaf.root.payload
        || leaf.root.key != source.context().digest.as_str()
    {
        return Err(invalid("discovered HTML capture descriptor differs").into());
    }
    let DiscoveredAttachmentLocatorV1::Inline { bytes, .. } = source.locator() else {
        return Ok(Err(Gap::UnsupportedLocatorMaterial));
    };
    grant.charge_steps(1).map_err(domain)?;
    Ok(OriginalResultSpan::parse(bytes).ok_or(Gap::UnsupportedLocatorSyntax))
}
