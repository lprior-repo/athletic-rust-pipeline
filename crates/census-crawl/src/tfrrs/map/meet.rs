use super::entity::push_identity;
use super::state::{Absorb, Page};
use crate::tfrrs::parse::{ParsedMeet, ParsedSection, PublishedDate};
use census_domain::model::{
    CanonicalMeet, CompetitionLevel, EventId, EventKind, Evidence, Gender, MeetId,
    RetainedConflict, SourceNamespace, Sport,
};

impl<'a> Absorb<'a> {
    pub(super) fn meet_for(
        &mut self,
        page: Page<'_>,
        published: &ParsedMeet,
        date: &PublishedDate,
        sport: Sport,
    ) -> crate::CrawlResult<MeetId> {
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
        retain_sport(meet, sport)?;
        Ok(id)
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

fn retain_sport(meet: &mut CanonicalMeet, sport: Sport) -> crate::CrawlResult<()> {
    if sport == Sport::Unknown {
        return Err(crate::CrawlError::Invariant {
            detail: "TFRRS publication does not establish meet sport".to_string(),
        });
    }
    if !meet.sports.contains(&sport) {
        meet.sports.push(sport);
        meet.sports.sort();
    }
    if meet.sports.len() > 1 {
        let detail = format!(
            "TFRRS publications disagree on meet sport: {:?}",
            meet.sports
        );
        let conflict = RetainedConflict::new("Meet sport", meet.id.as_str(), &meet.name, &detail);
        if !meet.retained_conflicts.contains(&conflict) {
            meet.retained_conflicts.push(conflict);
        }
        return Err(crate::CrawlError::Invariant { detail });
    }
    Ok(())
}
