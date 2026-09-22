//! The mints for the two entities a row's event names: the meet it was held at, and the event
//! itself.
//!
//! Meet identity is (state, date, name) and event identity (meet, kind, gender, division,
//! round): the host's numeric meet id and its standard-event handle are identity *channels*
//! recorded beside them, never hashed into the key.

use super::entity::push_identity;
use super::state::{Absorb, Page};
use crate::sources::tfrrs::parse::{ParsedMeet, ParsedSection, PublishedDate};
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CompetitionLevel, EventId, EventKind, Evidence, Gender, MeetId,
    SourceEventLabel, SourceNamespace,
};

impl<'a> Absorb<'a> {
    /// Resolve or mint the meet a row names.
    ///
    /// Meet identity is (state, date, name): the host's numeric meet id is an identity *channel*, not
    /// the canonical key, so a meet the same host publishes under two routes stays one meet.
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
    /// Resolve or mint the event one section publishes, with the kind the label mapped to.
    ///
    /// Event identity is (meet, kind, gender, division, round): the host's own standard-event handle
    /// is kept as a source label rather than hashed into the key, so the same event listed under two
    /// handles stays one event.
    pub(super) fn event_for(
        &mut self,
        page: Page<'_>,
        section: &ParsedSection,
        meet: &MeetId,
        gender: Gender,
    ) -> (EventId, EventKind) {
        let kind = EventKind::from_source_label(&section.label);
        if matches!(kind, EventKind::Unmapped { .. }) {
            self.stats.events_unmapped = self.stats.events_unmapped.saturating_add(1);
        }
        let mut minted = CanonicalEvent::new(meet, kind.clone(), gender, None, None);
        minted
            .evidence
            .push(Evidence::parsed(page.source.clone(), page.observed_on));
        let id = minted.id.clone();
        let event = self
            .accumulator
            .events
            .entry(id.as_str().to_string())
            .or_insert(minted);
        if !event
            .source_labels
            .iter()
            .any(|known| known.label == section.label)
        {
            event.source_labels.push(SourceEventLabel {
                source: page.source.clone(),
                label: section.label.clone(),
            });
        }
        (id, kind)
    }
}
