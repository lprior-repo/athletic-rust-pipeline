use super::{domain, DiscoveredIdentityLinkedHtmlResultDocumentV1};
use crate::participation_evidence::{invalid, recorded::read, result_context};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

impl DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_> {
    pub(in crate::participation_evidence::recorded) fn documentary_venue(
        &self,
        index: usize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<&str> {
        read::with_grant(grant, |grant| {
            let row = self
                .row(index)
                .ok_or_else(|| invalid("documentary venue row missing"))?;
            row.attachment().require_original_grant(grant)?;
            let document = self
                .checked_document()
                .as_milesplit()
                .ok_or_else(|| invalid("documentary venue source is unsupported"))?;
            if self.venue.get().is_none() {
                let bytes = document
                    .original_document()
                    .len()
                    .checked_mul(4)
                    .and_then(|value| value.checked_add(256))
                    .ok_or(census_store::StoreError::CounterOverflow)?;
                grant.charge_bytes(bytes).map_err(domain)?;
                let venue = result_context::documentary_result_venue(document)?;
                self.venue
                    .set(venue)
                    .map_err(|_| invalid("documentary venue initialized twice"))?;
            }
            self.venue
                .get()
                .map(String::as_str)
                .ok_or_else(|| invalid("documentary venue missing after parse").into())
        })
    }
}
