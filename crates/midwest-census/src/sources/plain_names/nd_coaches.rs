//! North Dakota (NDHSAA): offering/staff rows → canonical coach entities, plus the per-school
//! walk that fetches each member page, journals it and writes the rows.

use super::nd::{parse_nd_school_refs, NdOffering, NdSchoolRef, NdStaffRole};
use super::nd_walk::NdWalk;
use super::parse::{is_office_role, strip_honorific};
use super::{observed_on, Options, ND_ADAPTER_ID, ND_SCHOOLS_PHASE, ND_SCHOOLS_URL};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{
    CanonicalCoach, CoachId, CoachRole, Evidence, Gender, SchoolId, SourceRef, Sport,
};
use std::collections::HashSet;

/// Map an NDHSAA sport label onto our ontology plus the gender side it covers.
///
/// Handles the coop-suffixed labels by ignoring everything the caller did not already strip, and
/// returns `None` for non-TF/XC offerings (`Cheerleading`, `Wrestling`, `Music - Vocal`, …).
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

/// The coach/AD role a published NDHSAA staff label implies, or `None` for office staff.
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

/// Athletic/activities-director rows for one NDHSAA school, deduplicated by coach identity.
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

/// Cross-country / track coach rows for one NDHSAA school.
///
/// `CoachRole::Unknown` is deliberate: the table says "Coaches" and never distinguishes a head coach
/// from an assistant, so no split is invented here.
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

/// North Dakota half: index, then one page per member school.
pub(super) async fn collect_north_dakota(
    ctx: &AdapterContext<'_>,
    options: &Options,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> CrawlResult<(u64, u64)> {
    let Some(members) = nd_members(ctx, fetch, report).await? else {
        return Ok((0, 0));
    };
    let mut walk = NdWalk::new(observed_on(ctx, options));
    let journal = ctx.store.journal_keys(ND_SCHOOLS_PHASE)?;

    for member in &members {
        if walk.limit_reached(options.limit) {
            break;
        }
        let key = format!("ND:{}", member.id);
        if journal.contains(&key) {
            walk.note_resumed();
            continue;
        }
        walk.visit(ctx, fetch, report, member).await?;
    }

    walk.publish(report, members.len())
}

/// The NDHSAA school index's member links; `None` means the failure is on the report.
async fn nd_members(
    ctx: &AdapterContext<'_>,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> CrawlResult<Option<Vec<NdSchoolRef>>> {
    let index = match ctx.fetcher.get(ND_SCHOOLS_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("ndhsaa: {ND_SCHOOLS_URL} failed: {error}"));
            return Ok(None);
        }
    };
    let members = parse_nd_school_refs(&index.text())?;
    if members.is_empty() {
        report.errors = report.errors.saturating_add(1);
        report.note(format!(
            "ndhsaa: {ND_SCHOOLS_URL} carried no member-school links"
        ));
        return Ok(None);
    }
    Ok(Some(members))
}
