use super::{guarded, invalid, ResultSchoolScopeV1};
use crate::participation_evidence::recorded::{
    DiscoveredIdentityLinkedHtmlResultDocumentV1, DiscoveredResultAttachmentV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::SchoolId;
use census_domain::result_school_correspondence::{
    ResultSchoolReviewedRecordV1, ResultSchoolSourceClaimV1,
};
use census_store::{
    CurrentEvidenceSubjectAttachmentReferencesV1, CurrentResultSchoolCorrespondencesV1,
};

mod frozen;
mod source;
pub use frozen::checked_frozen;

pub struct CheckedResultSchoolCorrespondenceV1<'a> {
    record: &'a ResultSchoolReviewedRecordV1,
}

impl CheckedResultSchoolCorrespondenceV1<'_> {
    pub fn school(&self) -> &SchoolId {
        &self.record.command.source.target.school
    }

    pub fn record(&self) -> &ResultSchoolReviewedRecordV1 {
        self.record
    }
}

pub fn checked_current<'a>(
    current: &'a CurrentResultSchoolCorrespondencesV1<'_, '_>,
    attachments: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedResultSchoolCorrespondenceV1<'a>>> {
    guarded(grant, |grant| {
        current.require_original_grant(grant)?;
        let row = linked
            .row(index)
            .ok_or_else(|| invalid("checked school row absent"))?;
        row.attachment().require_original_grant(grant)?;
        if current.store_identity() != row.attachment().store_identity()
            || current.snapshot_sequence() != row.attachment().snapshot_sequence()
        {
            return Err(invalid("checked school current owner differs"));
        }
        let record = select(current.active_records(grant)?, row.attachment())?;
        let Some(record) = record else {
            return Ok(None);
        };
        super::original::current(attachments, row.attachment(), grant)?;
        source::require(
            source::Source::Current(current),
            linked,
            index,
            record,
            grant,
        )?;
        Ok(Some(CheckedResultSchoolCorrespondenceV1 { record }))
    })
}

fn select<'a>(
    records: impl Iterator<Item = &'a ResultSchoolReviewedRecordV1>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
) -> CrawlResult<Option<&'a ResultSchoolReviewedRecordV1>> {
    records
        .filter(|record| {
            let source = &record.command.source;
            source.person == *attached.target()
                && source.person_attachment.operation == attached.record().command.operation
                && source.person_attachment.digest == attached.record().record_digest
                && source.statement.row_locator == attached.subject().locator
                && source.statement.interpretation.digest == attached.subject().interpretation
        })
        .try_fold(None, |found, record| {
            if found.is_some() {
                return Err(invalid("ambiguous active school correspondence"));
            }
            Ok(Some(record))
        })
}

fn scope(source: &ResultSchoolSourceClaimV1) -> ResultSchoolScopeV1 {
    ResultSchoolScopeV1 {
        day: source.target.day.clone(),
        sport: source.target.sport,
        category: source.target.category,
        year: source.target.year,
    }
}
