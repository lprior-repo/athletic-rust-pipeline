use super::budget;
use super::map::{level_of, venue_state};
use super::{MeetRow, Options, ScheduleSport, ADAPTER_ID, BASE, PROVIDER};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult, PerformanceDateAssessment};
use census_domain::model::{CanonicalMeet, Evidence, SourceIdentity, SourceNamespace, SourceRef};
use census_domain::UsJurisdiction;

pub(super) struct Page<'a> {
    pub fetched: &'a FetchOutcome,
    pub sport: ScheduleSport,
    pub year: i16,
}

pub(super) fn project(
    ctx: &AdapterContext<'_>,
    options: &Options,
    page: &Page<'_>,
    row: &MeetRow,
) -> CrawlResult<Option<CanonicalMeet>> {
    match ctx.assess_performance_date(&row.date) {
        PerformanceDateAssessment::Future => return Ok(None),
        PerformanceDateAssessment::Unknown => {
            return Err(CrawlError::PerformanceDateUnknown {
                published: row.date.clone(),
                as_of: ctx.performance_as_of,
            })
        }
        PerformanceDateAssessment::Admitted => {}
    }
    let state = venue_state(&row.location)
        .ok_or_else(|| schema(page, "published venue geography is unknown"))?;
    if !options.jurisdictions.is_empty() && !options.jurisdictions.contains(&state) {
        return Ok(None);
    }
    admitted_meet(row, state, page).map(Some)
}

fn admitted_meet(
    row: &MeetRow,
    state: UsJurisdiction,
    page: &Page<'_>,
) -> CrawlResult<CanonicalMeet> {
    if row.name.is_empty() {
        return Err(schema(page, "published meet name is empty"));
    }
    let month = row
        .date
        .get(5..7)
        .ok_or_else(|| schema(page, "published month is absent"))?
        .parse::<u8>()
        .map_err(|_| schema(page, "published month is invalid"))?;
    let level = level_of(&row.name);
    let mut meet = CanonicalMeet::new(Some(state), row.name.clone(), row.date.clone(), level);
    meet.location = Some(row.location.clone());
    budget::reserve(&mut meet.sports, 1, 1)?;
    meet.sports.push(page.sport.sport_for(month));
    decorate(&mut meet, row.slug.clone(), page)?;
    Ok(meet)
}

fn schema(page: &Page<'_>, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: page.fetched.url.clone(),
        detail: detail.to_string(),
    }
}

fn decorate(meet: &mut CanonicalMeet, slug: Option<String>, page: &Page<'_>) -> CrawlResult<()> {
    budget::reserve(&mut meet.source_urls, 2, 2)?;
    if let Some(slug) = &slug {
        meet.source_urls.push(format!("{BASE}/links/{slug}"));
    }
    meet.source_urls.push(page.fetched.url.clone());
    budget::reserve(&mut meet.source_identities, 1, 1)?;
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: PROVIDER.to_string(),
        },
        match slug {
            Some(slug) => slug,
            None => format!("{}|{}", meet.date, meet.normalized_name),
        },
    ));
    let mut evidence = Evidence::parsed(
        SourceRef::new(ADAPTER_ID, Some(page.fetched.url.clone())),
        &page.fetched.fetched_at,
    );
    evidence.note = meet
        .location
        .as_ref()
        .map(|location| format!("provider schedule venue: {location}"));
    budget::reserve(&mut meet.evidence, 1, 1)?;
    meet.evidence.push(evidence);
    Ok(())
}
