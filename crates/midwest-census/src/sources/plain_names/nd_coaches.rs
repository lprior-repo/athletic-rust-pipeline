//! North Dakota (NDHSAA): offering/staff rows → canonical coach entities, plus the per-school
//! walk that fetches each member page, journals it and writes the rows.

use super::nd::{
    parse_nd_offerings, parse_nd_school_page, parse_nd_school_refs, parse_nd_staff, NdOffering,
    NdStaffRole,
};
use super::parse::{email_regex, is_office_role, strip_honorific};
use super::{
    observed_on, Options, ND_ADAPTER_ID, ND_COACHES_PHASE, ND_SCHOOLS_PHASE, ND_SCHOOLS_URL,
};
use crate::model::{
    CanonicalCoach, CanonicalSchool, CoachId, CoachRole, Evidence, Gender, SchoolId, SourceRef,
    Sport,
};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
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
) -> Result<(u64, u64)> {
    let index = match ctx.fetcher.get(ND_SCHOOLS_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("ndhsaa: {ND_SCHOOLS_URL} failed: {error}"));
            return Ok((0, 0));
        }
    };
    let members = parse_nd_school_refs(&index.text())?;
    if members.is_empty() {
        report.errors = report.errors.saturating_add(1);
        report.note(format!(
            "ndhsaa: {ND_SCHOOLS_URL} carried no member-school links"
        ));
        return Ok((0, 0));
    }

    let observed_on = observed_on(ctx, options);
    let journal = ctx.store.journal_keys(ND_SCHOOLS_PHASE)?;
    // Counters saturate: they only feed the report, and no source carries 2^64 rows.
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut processed = 0usize;
    let mut resumed = 0usize;
    let mut failed = 0usize;
    let mut ad_rows = 0usize;
    let mut slots = 0usize;
    let mut slots_named = 0usize;
    let mut pages_with_email = 0usize;

    for member in &members {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        let key = format!("ND:{}", member.id);
        if journal.contains(&key) {
            resumed = resumed.saturating_add(1);
            continue;
        }
        let url = member.url();
        let page = match ctx.fetcher.get(&url, fetch).await {
            Ok(outcome) => outcome,
            Err(error) => {
                failed = failed.saturating_add(1);
                report.errors = report.errors.saturating_add(1);
                report.note(format!("ndhsaa: {url} failed: {error}"));
                continue;
            }
        };
        let html = page.text();
        let Some((school, school_id)) = parse_nd_school_page(&html, member, &observed_on)? else {
            failed = failed.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            report.note(format!("ndhsaa: {url} carried no school heading"));
            continue;
        };

        if email_regex()?.is_match(&html) {
            pages_with_email = pages_with_email.saturating_add(1);
        }
        let staff = parse_nd_staff(&html)?;
        let offerings = parse_nd_offerings(&html)?;
        let ads = nd_ad_coaches(&staff, &school_id, &url, &observed_on);
        let sport_coaches = nd_sport_coaches(&offerings, &school_id, &url, &observed_on);
        for offering in &offerings {
            if parse_nd_sport(&offering.label).is_some() {
                slots = slots.saturating_add(1);
                if !offering.coaches.is_empty() {
                    slots_named = slots_named.saturating_add(1);
                }
            }
        }
        ad_rows = ad_rows.saturating_add(ads.len());
        let ad_count = ads.len();
        let sport_count = sport_coaches.len();
        coaches.extend(ads);
        coaches.extend(sport_coaches);
        schools.push(school);

        ctx.store.journal_done(
            ND_SCHOOLS_PHASE,
            &key,
            &serde_json::json!({ "slug": member.slug, "offerings": offerings.len() }),
        )?;
        ctx.store.journal_done(
            ND_COACHES_PHASE,
            &key,
            &serde_json::json!({
                "coach_rows": sport_count.saturating_add(ad_count),
                "ad_rows": ad_count
            }),
        )?;
        processed = processed.saturating_add(1);
    }

    let school_rows = u64::try_from(schools.len()).context("ndhsaa school count exceeds u64")?;
    let coach_rows = u64::try_from(coaches.len()).context("ndhsaa coach count exceeds u64")?;
    ctx.store
        .append_many(Table::Schools, &schools)
        .context("writing ndhsaa schools")?;
    ctx.store
        .append_many(Table::Coaches, &coaches)
        .context("writing ndhsaa coaches")?;

    report.note(format!(
        "ndhsaa: {processed} of {} member schools parsed ({resumed} already journalled, {failed} failed); \
         {coach_rows} coach rows ({ad_rows} athletic/activities directors); \
         TF/XC coach slots named {slots_named}/{slots}; {} listings",
        members.len(),
        members.len()
    ));
    report.note(format!(
        "ndhsaa: {pages_with_email} of {processed} school pages walked contain any email string; \
         names only: provider publishes no coach email (every coach entity has professional_email=None)"
    ));
    Ok((school_rows, coach_rows))
}
