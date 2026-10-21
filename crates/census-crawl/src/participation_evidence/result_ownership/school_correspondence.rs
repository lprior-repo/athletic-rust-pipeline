use crate::participation_evidence::recorded::{
    DiscoveredIdentityLinkedHtmlResultDocumentV1, RecordedProviderSchoolV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentRecordReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};
use census_domain::result_school_correspondence::ResultSchoolSourceClaimV1;
use census_store::{AttachmentContextObjectV1, CurrentEvidenceSubjectAttachmentReferencesV1};

use super::original;
use super::original::{domain, guarded, invalid, ownership, reference};
mod objects;
pub mod read;
mod statement;
mod target;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSchoolScopeV1 {
    pub day: ProfileDate,
    pub sport: Sport,
    pub category: Gender,
    pub year: SchoolYear,
}

pub struct ResultSchoolSourceProposalV1 {
    source: ResultSchoolSourceClaimV1,
    objects: objects::Objects,
}

impl ResultSchoolSourceProposalV1 {
    pub fn claim(&self) -> &ResultSchoolSourceClaimV1 {
        &self.source
    }

    pub fn borrowed_context(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Vec<AttachmentContextObjectV1<'_>>> {
        guarded(grant, |grant| self.objects.borrowed(grant))
    }
}

pub fn build_claim(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    provider: &RecordedProviderSchoolV1,
    scope: ResultSchoolScopeV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultSchoolSourceProposalV1> {
    guarded(grant, |grant| {
        let row = linked
            .row(index)
            .ok_or_else(|| invalid("school result row absent"))?;
        let attached = row.attachment();
        original::current(current, attached, grant)?;
        let recorded = original::reopen(current.snapshot(), linked, row.checked_row(), grant)?;
        let document =
            recorded.read_source(&[row.checked_row().locator()], row.checked_row().source())?;
        let checked = document
            .row(0)
            .ok_or_else(|| invalid("reopened result row absent"))?;
        original::subject(recorded.root(), &checked, attached, grant)?;
        target::scope_source(&document, &checked, &scope)?;
        let mut objects = objects::Objects::new(grant)?;
        let statement =
            statement::build_source(recorded.root(), &document, &checked, &mut objects, grant)?;
        let target = target::build(current.snapshot(), provider, scope, &mut objects, grant)?;
        let source = ResultSchoolSourceClaimV1 {
            statement,
            target,
            person: attached.target().clone(),
            person_attachment: AttachmentRecordReferenceV1 {
                operation: attached.record().command.operation.clone(),
                digest: attached.record().record_digest.clone(),
            },
            person_inventory: attached.inventory().clone(),
            evidence: objects.references(grant)?,
        };
        Ok(ResultSchoolSourceProposalV1 { source, objects })
    })
}

#[cfg(test)]
mod scenario;
