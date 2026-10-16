use super::{domain, read, view, DiscoveredIdentityLinkedHtmlResultDocumentV1};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    ActiveResultSubjectDiscoveryV1, CurrentEvidenceSubjectAttachmentReferencesV1,
    FrozenCurrentEvidenceSubjectAttachmentReferencesV1,
};

mod classify;
mod consume;
mod grouping;
mod types;

pub use types::{AttachmentHtmlResultDiscoveryGapV1, UnresolvedAttachmentHtmlResultV1};
use types::{Diagnostic, Member};

pub struct AttachmentHtmlResultDiscoveryV1<'a, 'v, 's> {
    view: view::View<'a, 'v, 's>,
    metadata: ActiveResultSubjectDiscoveryV1<'a>,
    members: Vec<Member>,
    groups: Vec<std::ops::Range<usize>>,
    diagnostics: Vec<Diagnostic>,
}

pub fn discover_current_html_results<'a, 'v, 's>(
    current: &'a CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<AttachmentHtmlResultDiscoveryV1<'a, 'v, 's>> {
    discover(view::View::Current(current), grant)
}

pub fn discover_frozen_html_results<'a, 's>(
    current: &'a FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<AttachmentHtmlResultDiscoveryV1<'a, 'a, 's>> {
    discover(view::View::Frozen(current), grant)
}

fn discover<'a, 'v, 's>(
    view: view::View<'a, 'v, 's>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<AttachmentHtmlResultDiscoveryV1<'a, 'v, 's>> {
    read::with_grant(grant, |grant| {
        let metadata = view.discover(grant)?;
        let (members, diagnostics) = classify::all(&metadata, grant)?;
        let groups = grouping::ranges(&metadata, &members, grant)?;
        Ok(AttachmentHtmlResultDiscoveryV1 {
            view,
            metadata,
            members,
            groups,
            diagnostics,
        })
    })
}

impl<'a, 'v, 's> AttachmentHtmlResultDiscoveryV1<'a, 'v, 's> {
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
    pub fn diagnostic_count(&self) -> usize {
        self.diagnostics.len()
    }
    pub fn diagnostic(&self, index: usize) -> Option<UnresolvedAttachmentHtmlResultV1<'_, 'a>> {
        let diagnostic = self.diagnostics.get(index)?;
        Some(UnresolvedAttachmentHtmlResultV1 {
            source: self.metadata.subjects().get(diagnostic.index)?,
            reason: diagnostic.reason,
        })
    }
    pub fn with_group<T>(
        &self,
        index: usize,
        grant: &mut AttachmentReplayGrantV1,
        consume: impl for<'r> FnOnce(
            DiscoveredIdentityLinkedHtmlResultDocumentV1<'r, 's>,
            &mut AttachmentReplayGrantV1,
        ) -> CrawlResult<T>,
    ) -> CrawlResult<T> {
        read::with_grant(grant, |grant| {
            let group = self
                .groups
                .get(index)
                .ok_or_else(|| invalid("result group absent"))?;
            let members = self
                .members
                .get(group.clone())
                .ok_or_else(|| invalid("result group range differs"))?;
            consume::group(self.view, &self.metadata, members, grant, consume)
        })
    }
}

fn reserve<T>(count: usize, grant: &mut AttachmentReplayGrantV1) -> CrawlResult<Vec<T>> {
    let limit = census_domain::public_identity_history::ReplayLimits::SMALL.traversal_steps;
    if count > limit {
        return Err(domain(
            census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Capacity {
                resource: "discovery metadata slots",
                needed: count,
                available: limit,
            },
        ));
    }
    count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant.charge_steps(count).map_err(domain)?;
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| {
        domain(
            census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Allocation,
        )
    })?;
    Ok(values)
}

fn invalid(detail: &'static str) -> census_store::StoreError {
    super::super::super::invalid(detail)
}

#[cfg(test)]
pub(in crate::participation_evidence::recorded) mod tests;
