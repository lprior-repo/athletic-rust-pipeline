//! Nebraska (NSAA): directory rows → canonical school/coach entities, plus the per-school walk
//! that fetches each member page, journals it and writes the rows.

use super::nsaa::{
    nsaa_school_url, parse_nsaa_directory, parse_nsaa_school_names, NsaaRow, NsaaSchool,
};
use super::parse::{email_regex, is_office_role, split_person_names};
use super::{
    observed_on, Options, NSAA_ADAPTER_ID, NSAA_COACHES_PHASE, NSAA_FORM_URL, NSAA_SCHOOLS_PHASE,
};
use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachId, CoachRole, Evidence, Gender,
    SchoolId, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
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
    let form = match ctx.fetcher.get(NSAA_FORM_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("nsaa: {NSAA_FORM_URL} failed: {error}"));
            return Ok((0, 0));
        }
    };
    let members = parse_nsaa_school_names(&form.text())?;
    if members.is_empty() {
        report.errors = report.errors.saturating_add(1);
        report.note(format!(
            "nsaa: {NSAA_FORM_URL} carried no member-school options"
        ));
        return Ok((0, 0));
    }

    let observed_on = observed_on(ctx, options);
    let journal = ctx.store.journal_keys(NSAA_SCHOOLS_PHASE)?;
    // Counters saturate: they only feed the report, and no source carries 2^64 rows.
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut processed = 0usize;
    let mut resumed = 0usize;
    let mut failed = 0usize;
    let mut ad_rows = 0usize;
    let mut sport_rows = 0usize;
    let mut slots = 0usize;
    let mut slots_named = 0usize;
    let mut rows_with_email = 0usize;
    let mut coach_rows_with_email = 0usize;
    let mut role_rows = 0usize;

    for name in &members {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        let key = format!("NE:{name}");
        if journal.contains(&key) {
            resumed = resumed.saturating_add(1);
            continue;
        }
        let url = nsaa_school_url(name);
        let page = match ctx.fetcher.get(&url, fetch).await {
            Ok(outcome) => outcome,
            Err(error) => {
                failed = failed.saturating_add(1);
                report.errors = report.errors.saturating_add(1);
                report.note(format!("nsaa: {url} failed: {error}"));
                continue;
            }
        };
        let blocks = parse_nsaa_directory(&page.text())?;
        let Some(entry) = blocks
            .iter()
            .find(|entry| &entry.name == name)
            .or_else(|| blocks.first())
        else {
            failed = failed.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            report.note(format!("nsaa: {url} carried no school block"));
            continue;
        };

        let (school, school_id) = parse_nsaa_school(entry, &url, &observed_on);
        let school_coaches = nsaa_coaches(entry, &school_id, &url, &observed_on)?;
        for role in &entry.roles {
            role_rows = role_rows.saturating_add(1);
            if email_regex()?.is_match(&role.name) {
                rows_with_email = rows_with_email.saturating_add(1);
                if parse_nsaa_row(&role.label).is_some() {
                    coach_rows_with_email = coach_rows_with_email.saturating_add(1);
                }
            }
            if matches!(
                parse_nsaa_row(&role.label),
                Some(NsaaRow::SportCoach { .. })
            ) {
                slots = slots.saturating_add(1);
                if !split_person_names(&role.name)?.is_empty() {
                    slots_named = slots_named.saturating_add(1);
                }
            }
        }
        ad_rows = ad_rows.saturating_add(
            school_coaches
                .iter()
                .filter(|coach| coach.role == CoachRole::AthleticDirector)
                .count(),
        );
        sport_rows = sport_rows.saturating_add(
            school_coaches
                .iter()
                .filter(|coach| coach.role == CoachRole::HeadCoach)
                .count(),
        );
        let coach_count = school_coaches.len();
        coaches.extend(school_coaches);
        schools.push(school);

        ctx.store.journal_done(
            NSAA_SCHOOLS_PHASE,
            &key,
            &serde_json::json!({ "published_rows": entry.roles.len() }),
        )?;
        ctx.store.journal_done(
            NSAA_COACHES_PHASE,
            &key,
            &serde_json::json!({ "coach_rows": coach_count }),
        )?;
        processed = processed.saturating_add(1);
    }

    let school_rows = u64::try_from(schools.len()).context("nsaa school count exceeds u64")?;
    let coach_rows = u64::try_from(coaches.len()).context("nsaa coach count exceeds u64")?;
    ctx.store
        .append_many(Table::Schools, &schools)
        .context("writing nsaa schools")?;
    ctx.store
        .append_many(Table::Coaches, &coaches)
        .context("writing nsaa coaches")?;

    report.note(format!(
        "nsaa: {processed} of {} member schools parsed ({resumed} already journalled, {failed} failed); \
         {coach_rows} coach rows ({ad_rows} athletic/activities directors, {sport_rows} sport rows); \
         TF/XC coach slots named {slots_named}/{slots}",
        members.len()
    ));
    report.note(format!(
        "nsaa: {rows_with_email} of {role_rows} directory rows carry an email string \
         ({coach_rows_with_email} of them in coach/AD rows); \
         names only: provider publishes no coach email"
    ));
    Ok((school_rows, coach_rows))
}
