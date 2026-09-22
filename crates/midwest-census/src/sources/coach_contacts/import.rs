//! The CSV walk itself: read each record, fold its entities into the accumulated maps (collapsing
//! the duplicate AD rows), and write the schools and coaches through the store.

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::path::Path;

use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachId};
use census_domain::UsJurisdiction;

use crate::sources::{AdapterReport, CrawlError, CrawlResult};
use crate::store::{Store, Table};

use super::entities::row_entities;
use super::wire::{CoachContactRow, RowEntities};

/// Import a contact CSV, writing canonical schools and coaches into the store.
///
/// Duplicate coach rows (the same AD repeated across a school's sport rows) collapse to one entity
/// with the union of its evidence and the first non-empty email.
pub fn import_csv(
    store: &Store,
    csv_path: &Path,
    default_observed_on: &str,
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("coach_contacts_csv", "coaches");
    let file = std::fs::File::open(csv_path).map_err(|source| CrawlError::Io {
        path: csv_path.to_path_buf(),
        source,
    })?;
    let mut reader = csv::Reader::from_reader(file);

    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    let mut coaches: BTreeMap<CoachId, CanonicalCoach> = BTreeMap::new();
    let mut skipped_roles = 0usize;

    for (index, record) in reader.deserialize::<CoachContactRow>().enumerate() {
        let (row, state) = contact_row(record, index, csv_path)?;
        let entities = row_entities(&row, state, default_observed_on)?;
        if merge_entities(&mut schools, &mut coaches, entities) {
            skipped_roles = skipped_roles.saturating_add(1);
        }
    }
    let _ = Table::Coaches;

    write_entities(store, &mut report, schools, coaches, skipped_roles)?;
    Ok(report)
}

/// Deserialize one CSV record into a row plus the jurisdiction it names.
///
/// The CSV is an operator-supplied file, so this is the one place its state column becomes a
/// [`UsJurisdiction`]: a row that names no school, no state, or a state outside the census is
/// refused with its line number rather than filed under a guess.
fn contact_row(
    record: Result<CoachContactRow, csv::Error>,
    index: usize,
    csv_path: &Path,
) -> CrawlResult<(CoachContactRow, UsJurisdiction)> {
    let line = index.saturating_add(2);
    let row = record.map_err(|source| CrawlError::Schema {
        url: csv_path.display().to_string(),
        detail: format!("row {line}: {source}"),
    })?;
    if row.school.trim().is_empty() || row.state.trim().is_empty() {
        return Err(CrawlError::Schema {
            url: csv_path.display().to_string(),
            detail: format!("row {line} has no school/state"),
        });
    }
    let state = UsJurisdiction::from_code(row.state.trim()).ok_or_else(|| CrawlError::Schema {
        url: csv_path.display().to_string(),
        detail: format!(
            "row {line}: `{}` is not one of the 50 states or the District of Columbia",
            row.state.trim()
        ),
    })?;
    Ok((row, state))
}

/// Fold one row's entities into the accumulated schools and coaches.
///
/// Returns `true` when the row carried no coach role.
fn merge_entities(
    schools: &mut BTreeMap<String, CanonicalSchool>,
    coaches: &mut BTreeMap<CoachId, CanonicalCoach>,
    entities: RowEntities,
) -> bool {
    let school_id = entities.school.id.clone();
    schools
        .entry(school_id.as_str().to_string())
        .or_insert(entities.school);
    let without_coach_role = entities.coaches.is_empty();
    for coach in entities.coaches {
        merge_coach(coaches, coach);
    }
    without_coach_role
}

/// Union one coach row into the map entry it shares an identity with.
fn merge_coach(coaches: &mut BTreeMap<CoachId, CanonicalCoach>, coach: CanonicalCoach) {
    match coaches.get_mut(&coach.id) {
        Some(existing) => {
            if existing.professional_email.is_none() {
                existing.professional_email = coach.professional_email.clone();
            }
            for evidence in coach.evidence.iter().cloned() {
                if !existing.evidence.contains(&evidence) {
                    existing.evidence.push(evidence);
                }
            }
            for identity in coach.source_identities.iter().cloned() {
                if !existing.source_identities.contains(&identity) {
                    existing.source_identities.push(identity);
                }
            }
        }
        None => {
            coaches.insert(coach.id.clone(), coach);
        }
    }
}

/// Write the accumulated entities and fill in the report's counters and notes.
fn write_entities(
    store: &Store,
    report: &mut AdapterReport,
    schools: BTreeMap<String, CanonicalSchool>,
    coaches: BTreeMap<CoachId, CanonicalCoach>,
    skipped_roles: usize,
) -> CrawlResult<()> {
    let school_records: Vec<CanonicalSchool> = schools.into_values().collect();
    let coach_records: Vec<CanonicalCoach> = coaches.into_values().collect();
    store.append_many(Table::Schools, &school_records)?;
    store.append_many(Table::Coaches, &coach_records)?;

    report.rows = u64::try_from(coach_records.len()).unwrap_or(u64::MAX);
    report.with_email = coach_records
        .iter()
        .filter(|coach| coach.professional_email.is_some())
        .count()
        .try_into()
        .unwrap_or(u64::MAX);
    report.note(format!("schools={}", school_records.len()));
    report.note(format!("rows_without_coach_role={skipped_roles}"));
    Ok(())
}
