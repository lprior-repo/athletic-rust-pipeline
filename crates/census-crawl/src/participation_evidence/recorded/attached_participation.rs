use super::attached_result::view::View;
use super::attached_result::DiscoveredIdentityLinkedHtmlResultDocumentV1;
use crate::participation_evidence::{invalid, VerifiedIndependentParticipationV1};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_store::{
    CurrentEvidenceSubjectAttachmentReferencesV1, FrozenCurrentEvidenceSubjectAttachmentReferencesV1,
};

mod join;
mod read;
mod subject;
mod types;

pub use types::{
    AttachedParticipationV1, CheckedDocumentaryParticipationFactsV1,
    CheckedDocumentarySourceFactsV1, DocumentarySeasonApplicabilityV1, DocumentaryCohortAgreementV1,
};
use types::{CheckedAttachedStatusV1, StatusBinding};

pub fn with_current_attached_participation<'a, 'v, 's, T>(
    owner: &'a CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
    grant: &mut Grant,
    consume: impl FnOnce(&AttachedParticipationV1<'a, 's>, &mut Grant) -> CrawlResult<T>,
) -> CrawlResult<T> {
    with_sources(View::Current(owner), grant, consume)
}

pub fn with_frozen_attached_participation<'a, 's, T>(
    owner: &'a FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'s>,
    grant: &mut Grant,
    consume: impl FnOnce(&AttachedParticipationV1<'a, 's>, &mut Grant) -> CrawlResult<T>,
) -> CrawlResult<T> {
    with_sources(View::Frozen(owner), grant, consume)
}

fn with_sources<'a, 'v, 's, T>(
    view: View<'a, 'v, 's>,
    grant: &mut Grant,
    consume: impl FnOnce(&AttachedParticipationV1<'a, 's>, &mut Grant) -> CrawlResult<T>,
) -> CrawlResult<T> {
    super::read::with_grant(grant, |grant| {
        let sources = read::sources(view, grant)?;
        consume(&sources, grant)
    })
}

impl<'s> AttachedParticipationV1<'_, 's> {
    pub fn with_row_facts<T>(
        &self,
        linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
        index: usize,
        grant: &mut Grant,
        consume: impl for<'f> FnOnce(CheckedDocumentaryParticipationFactsV1<'f>, &mut Grant) -> CrawlResult<T>,
    ) -> CrawlResult<T> {
        super::read::with_grant(grant, |grant| {
            let facts = join::row(self, linked, index, grant)?;
            consume(CheckedDocumentaryParticipationFactsV1 { sources: &facts }, grant)
        })
    }
}

fn steps(grant: &mut Grant, count: usize) -> CrawlResult<()> {
    grant.charge_steps(count).map_err(census_store::EvidenceSubjectAttachmentStoreError::from)
        .map_err(census_store::StoreError::from)?;
    Ok(())
}

fn bytes(grant: &mut Grant, count: usize) -> CrawlResult<()> {
    grant.charge_bytes(count).map_err(census_store::EvidenceSubjectAttachmentStoreError::from)
        .map_err(census_store::StoreError::from)?;
    Ok(())
}

fn reserve<T>(values: &mut Vec<T>, grant: &mut Grant) -> CrawlResult<()> {
    bytes(grant, std::mem::size_of::<T>())?;
    values.try_reserve_exact(1).map_err(|_| invalid("attached participation allocation refused"))?;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
