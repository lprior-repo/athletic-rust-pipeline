//! Nebraska (NSAA): directory rows → canonical school/coach entities, plus the per-school walk
//! that fetches each member page, journals it and writes the rows.

use super::nsaa::{parse_nsaa_school_names, NsaaRow, NsaaSchool};
use super::nsaa_walk::NsaaWalk;
use super::parse::{is_office_role, split_person_names};
use super::{observed_on, Options, NSAA_ADAPTER_ID, NSAA_FORM_URL, NSAA_SCHOOLS_PHASE};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use anyhow::Result;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachId, CoachRole, Evidence, Gender,
    SchoolId, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use std::collections::HashSet;

/// Classify one NSAA directory row label.
///
/// Office roles are rejected first, so an `AD Secretary` never reaches the director branch. The
/// `Unified Track & Field` activity is a distinct NSAA offering (Special Olympics unified) and is not
/// the census's track & field sport, so it is excluded as well.
pub fn parse_nsaa_row(label: &str) -> Option<NsaaRow> {
    let lowered = label.to_ascii_lowercase();
    if is_office_role(&lowered) {
        return None;
    }
    if lowered.contains("athletic director") || lowered.contains("activities director") {
        return Some(NsaaRow::AthleticDirector);
    }
    if lowered.contains("unified") {
        return None;
    }
    let sport = if lowered.contains("cross-country") || lowered.contains("cross country") {
        Sport::CrossCountry
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
    Some(NsaaRow::SportCoach { sport, gender })
}

/// Canonical school for one NSAA directory entry. NSAA publishes no numeric school id, so the
/// published school name is the provider key (recorded in the association-school namespace).
pub fn parse_nsaa_school(
    school: &NsaaSchool,
    source_url: &str,
    observed_on: &str,
) -> (CanonicalSchool, SchoolId) {
    let (mut canonical, school_id) =
        CanonicalSchool::new("NE", &school.name, normalize_name(&school.name));
    canonical.city = school.city.clone();
    canonical.enrollment = school.enrollment;
    canonical.school_website = school.homepage.clone();
    canonical.association = Some(NSAA_ADAPTER_ID.to_string());
    canonical.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: NSAA_ADAPTER_ID.to_string(),
            },
            &school.name,
        )
        .with_url(source_url),
    );
    canonical.evidence.push(Evidence::parsed(
        SourceRef::new(NSAA_ADAPTER_ID, Some(source_url.to_string())),
        observed_on,
    ));
    (canonical, school_id)
}

/// Coach and athletic-director rows for one NSAA school, deduplicated by coach identity.
pub fn nsaa_coaches(
    school: &NsaaSchool,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Result<Vec<CanonicalCoach>> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<CoachId> = HashSet::new();
    for role in &school.roles {
        let Some(kind) = parse_nsaa_row(&role.label) else {
            continue;
        };
        for name in split_person_names(&role.name)? {
            let mut coach = match kind {
                NsaaRow::SportCoach { sport, gender } => {
                    CanonicalCoach::new(school_id, name, Some(sport), gender, CoachRole::HeadCoach)
                }
                NsaaRow::AthleticDirector => CanonicalCoach::new(
                    school_id,
                    name,
                    None,
                    Gender::Mixed,
                    CoachRole::AthleticDirector,
                ),
            };
            coach.evidence.push(Evidence::parsed(
                SourceRef::new(NSAA_ADAPTER_ID, Some(source_url.to_string())),
                observed_on,
            ));
            if seen.insert(coach.id.clone()) {
                coaches.push(coach);
            }
        }
    }
    Ok(coaches)
}

/// Nebraska half: the directory form yields the 312 member names, then one GET per school.
pub(super) async fn collect_nebraska(
    ctx: &AdapterContext<'_>,
    options: &Options,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> Result<(u64, u64)> {
    let Some(members) = nsaa_members(ctx, fetch, report).await? else {
        return Ok((0, 0));
    };
    let mut walk = NsaaWalk::new(observed_on(ctx, options));
    let journal = ctx.store.journal_keys(NSAA_SCHOOLS_PHASE)?;

    for name in &members {
        if walk.limit_reached(options.limit) {
            break;
        }
        let key = format!("NE:{name}");
        if journal.contains(&key) {
            walk.note_resumed();
            continue;
        }
        walk.visit(ctx, fetch, report, name).await?;
    }

    walk.publish(ctx, report, members.len())
}

/// The NSAA directory form's member-school names; `None` means the failure is on the report.
async fn nsaa_members(
    ctx: &AdapterContext<'_>,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> Result<Option<Vec<String>>> {
    let form = match ctx.fetcher.get(NSAA_FORM_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("nsaa: {NSAA_FORM_URL} failed: {error}"));
            return Ok(None);
        }
    };
    let members = parse_nsaa_school_names(&form.text())?;
    if members.is_empty() {
        report.errors = report.errors.saturating_add(1);
        report.note(format!(
            "nsaa: {NSAA_FORM_URL} carried no member-school options"
        ));
        return Ok(None);
    }
    Ok(Some(members))
}
