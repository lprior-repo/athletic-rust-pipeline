use super::super::{binding, invalid, objects::Objects, ownership, statement, target};
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1;
use crate::participation_evidence::result_ownership::read_retained_html_result_rows;
use crate::participation_evidence::RetainedHtmlCaptureV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::CanonicalSchool;
use census_domain::result_school_correspondence::{
    ResultSchoolDigestV1, ResultSchoolReviewedRecordV1,
};
use census_store::{
    CurrentResultSchoolCorrespondencesV1, ReplayedResultSchoolCorrespondencesV1,
    RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1,
};

#[derive(Clone, Copy)]
pub(super) enum Source<'a, 'v, 's> {
    Current(&'a CurrentResultSchoolCorrespondencesV1<'v, 's>),
    Frozen(&'a ReplayedResultSchoolCorrespondencesV1<'s>),
}

impl Source<'_, '_, '_> {
    fn root(
        self,
        schema: RetainedCaptureRootSchemaV1,
        key: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<RetainedCaptureRootReadV1> {
        let root = match self {
            Self::Current(current) => current
                .snapshot()
                .read_retained_capture_root_v1(schema, key, grant)?,
            Self::Frozen(current) => current.read_retained_capture_root_v1(schema, key, grant)?,
        };
        root.ok_or_else(|| invalid("reviewed school original capture absent"))
    }

    fn provider_root(
        self,
        digest: &ResultSchoolDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<RetainedCaptureRootV1> {
        Ok(match self {
            Self::Current(current) => current.provider_school_root(digest, grant)?,
            Self::Frozen(current) => current.provider_school_root(digest, grant)?,
        })
    }

    fn physical_school(
        self,
        digest: &ResultSchoolDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<CanonicalSchool> {
        Ok(match self {
            Self::Current(current) => current.physical_school(digest, grant)?,
            Self::Frozen(current) => current.physical_school(digest, grant)?,
        })
    }
}

pub(super) fn require(
    source: Source<'_, '_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    record: &ResultSchoolReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let linked_row = linked
        .row(index)
        .ok_or_else(|| invalid("reviewed school original row absent"))?;
    let original = source.root(
        RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        &linked.recorded_root().key,
        grant,
    )?;
    if original.root() != linked.recorded_root() {
        return Err(invalid("reviewed school original result root differs"));
    }
    let capture: RetainedHtmlCaptureV1 = serde_json::from_str(original.payload().get())
        .map_err(crate::participation_evidence::json)?;
    let document = read_retained_html_result_rows(&capture, &[linked_row.checked_row().locator()])
        .map_err(ownership)?;
    let row = document
        .row(0)
        .ok_or_else(|| invalid("reviewed school original selected row absent"))?;
    binding::subject(original.root(), &row, linked_row.attachment(), grant)?;
    let scope = super::scope(&record.command.source);
    target::scope(&document, &row, &scope)?;
    let mut objects = Objects::new(grant)?;
    let statement = statement::build(original.root(), &document, &row, &mut objects, grant)?;
    let target = provider(source, record, scope, &mut objects, grant)?;
    let accepted = &record.command.source;
    if accepted.statement != statement
        || accepted.target != target
        || accepted.evidence != objects.references(grant)?
    {
        return Err(invalid(
            "reviewed school differs from original source recheck",
        ));
    }
    Ok(())
}

fn provider(
    source: Source<'_, '_, '_>,
    record: &ResultSchoolReviewedRecordV1,
    scope: super::super::ResultSchoolScopeV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<census_domain::result_school_correspondence::ResultSchoolTargetClaimV1> {
    let root = source.provider_root(&record.record_digest, grant)?;
    let original = source.root(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &root.key,
        grant,
    )?;
    if original.root() != &root {
        return Err(invalid("reviewed school original provider root differs"));
    }
    let packet: ProviderSchoolPacketV1 = serde_json::from_str(original.payload().get())
        .map_err(crate::participation_evidence::json)?;
    let checked = packet.read_semantic_contract_v1()?;
    if checked.reference().as_str() != root.key {
        return Err(invalid("reviewed school original provider key differs"));
    }
    let physical = source.physical_school(&record.record_digest, grant)?;
    target::checked(&root, &checked, &physical, scope, objects, grant)
}
