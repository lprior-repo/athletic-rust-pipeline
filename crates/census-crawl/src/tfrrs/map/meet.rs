use super::entity::push_identity;
use super::state::{Absorb, Page};
use crate::tfrrs::parse::{ParsedMeet, ParsedSection, PublishedDate};
use census_domain::model::{
    CanonicalMeet, CompetitionLevel, EventId, EventKind, Evidence, Gender, MeetId, SourceNamespace,
};

impl<'a> Absorb<'a> {
    pub(super) fn meet_for(
        &mut self,
        page: Page<'_>,
        published: &ParsedMeet,
        date: &PublishedDate,
    ) -> MeetId {
        let id = CanonicalMeet::mint(Some(page.jurisdiction), &date.iso, &published.name, None);
        let meet = self
            .accumulator
            .meets
            .entry(id.as_str().to_string())
            .or_insert_with(|| {
                CanonicalMeet::new(
                    Some(page.jurisdiction),
                    published.name.clone(),
                    date.iso.clone(),
                    CompetitionLevel::Unknown,
                )
            });
        meet.evidence
            .push(Evidence::parsed(page.source.clone(), page.observed_on));
        if let Some(url) = published.path.as_deref() {
            if !meet.source_urls.iter().any(|known| known == url) {
                meet.source_urls.push(url.to_string());
            }
        }
        if let Some(meet_id) = published.id {
            push_identity(
                &mut meet.source_identities,
                SourceNamespace::TfrrsMeet,
                &meet_id.to_string(),
                published.path.clone(),
            );
        }
        id
    }
    pub(super) fn event_for(
        &mut self,
        page: Page<'_>,
        section: &ParsedSection,
        meet: &MeetId,
        gender: Gender,
    ) -> crate::CrawlResult<(EventId, EventKind)> {
        let minted = super::events::mint(meet, gender, section)?;
        let kind = minted.kind.clone();
        if matches!(kind, EventKind::Unmapped { .. }) {
            self.stats.events_unmapped = self.stats.events_unmapped.saturating_add(1);
        }
        let id = super::events::retain(&mut self.accumulator, minted, page, &section.label)?;
        Ok((id, kind))
    }
}
