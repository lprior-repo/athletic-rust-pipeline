use super::nd::{parse_nd_school_refs, NdOffering, NdSchoolRef, NdStaffRole};
use super::parse::{is_office_role, strip_honorific};
use super::{Options, ND_ADAPTER_ID, ND_SCHOOLS_URL};
use crate::directory::acquisition::{fail, owe, text};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalCoach, CoachId, CoachRole, Evidence, Gender, SchoolId, SourceRef, Sport,
};
use futures::{stream, StreamExt, TryStreamExt};
use std::collections::HashSet;

pub fn parse_nd_sport(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.to_ascii_lowercase();
    let sport = if lowered.contains("cross country") || lowered.contains("cross-country") {
        Sport::CrossCountry
    } else if lowered.contains("indoor") && lowered.contains("track") {
        Sport::IndoorTrack
    } else if lowered.contains("track") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if lowered.contains("girls") {
        Gender::Girls
    } else if lowered.contains("boys") {
        Gender::Boys
    } else {
        Gender::Mixed
    };
    Some((sport, gender))
}

pub fn parse_nd_role(label: &str) -> Option<CoachRole> {
    let lowered = label.to_ascii_lowercase();
    if is_office_role(&lowered) {
        return None;
    }
    if lowered.contains("athletic director") || lowered.contains("activities director") {
        return Some(CoachRole::AthleticDirector);
    }
    None
}

pub fn nd_ad_coaches(
    roles: &[NdStaffRole],
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<CoachId> = HashSet::new();
    for role in roles {
        let Some(kind) = parse_nd_role(&role.label) else {
            continue;
        };
        let name = strip_honorific(&role.name);
        if name.is_empty() {
            continue;
        }
        let mut coach = CanonicalCoach::new(school_id, name, None, Gender::Mixed, kind);
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(ND_ADAPTER_ID, Some(source_url.to_string())),
            observed_on,
        ));
        if seen.insert(coach.id.clone()) {
            coaches.push(coach);
        }
    }
    coaches
}

pub fn nd_sport_coaches(
    offerings: &[NdOffering],
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<CoachId> = HashSet::new();
    for offering in offerings {
        let Some((sport, gender)) = parse_nd_sport(&offering.label) else {
            continue;
        };
        for name in &offering.coaches {
            let mut coach = CanonicalCoach::new(
                school_id,
                name.clone(),
                Some(sport),
                gender,
                CoachRole::Unknown,
            );
            coach.evidence.push(Evidence::parsed(
                SourceRef::new(ND_ADAPTER_ID, Some(source_url.to_string())),
                observed_on,
            ));
            if seen.insert(coach.id.clone()) {
                coaches.push(coach);
            }
        }
    }
    coaches
}

pub(super) async fn collect_north_dakota(
    ctx: &AdapterContext<'_>,
    options: &Options,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> CrawlResult<(u64, u64)> {
    let Some(members) = nd_members(ctx, fetch, report).await? else {
        return Ok((0, 0));
    };
    let result = stream::iter(members.iter().enumerate())
        .map(Ok::<_, CrawlError>)
        .try_fold(
            (report, 0usize, 0usize),
            |(report, schools, coaches), (ordinal, member)| async move {
                if options.limit.is_some_and(|limit| ordinal >= limit) {
                    owe(report, member.url())?;
                    return Ok((report, schools, coaches));
                }
                let written = super::nd_walk::visit(ctx, fetch, report, member).await?;
                Ok((
                    report,
                    schools.saturating_add(written.0),
                    coaches.saturating_add(written.1),
                ))
            },
        )
        .await?;
    Ok((
        u64::try_from(result.1).map_or(u64::MAX, |value| value),
        u64::try_from(result.2).map_or(u64::MAX, |value| value),
    ))
}

async fn nd_members(
    ctx: &AdapterContext<'_>,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> CrawlResult<Option<Vec<NdSchoolRef>>> {
    let index = match ctx.fetcher.get(ND_SCHOOLS_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            fail(report, ND_SCHOOLS_URL, error)?;
            return Ok(None);
        }
    };
    let members = match text(&index).and_then(parse_nd_school_refs) {
        Ok(members) => members,
        Err(error) => {
            fail(report, ND_SCHOOLS_URL, error)?;
            return Ok(None);
        }
    };
    if members.is_empty() {
        owe(report, ND_SCHOOLS_URL)?;
    }
    Ok(Some(members))
}
