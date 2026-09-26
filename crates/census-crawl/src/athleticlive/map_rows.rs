//! One mapped result row: the school it resolves to, the athlete it observes, and the performance it
//! mints.
//!
//! Both routes that can publish a race read their rows through this file — an event document
//! (`record_row`) and a live-standings payload (`record_standing`) — so the two agree on every
//! identity they mint, including the performance key (the event's key plus the athlete, never the
//! row's position, which differs between the two payloads).
//!
//! Refusals are counted by reason and contribute no entity: a row with no published name, no school
//! label, no high-school grade, or a label that names no consolidated school. A row that maps but
//! publishes no mark (`NH`) still yields its athlete and team: the identity is evidence even when the
//! mark is not.

use super::docs::{value_u64, DocRow, DocTeam};
use super::map::{ResultStats, RowContext, Writer};
use super::standings::StandingRow;
use crate::athleticlive_athletes::{gender_from_token, grade_from_token};
use census_domain::model::{AthleteId, Gender, Grade, TeamId};
use serde_json::Value;

mod identity;
mod performance;

use identity::{map_identity, resolve_school};
use performance::{write_performance, PerformanceFacts};

/// True for the reasons a row can be refused before any entity is minted.
enum Refusal {
    NoName,
    NoSchool,
    BelowHighSchool,
    NoGrade,
}

/// The facts one row publishes about its athlete and team, as the mapping needs them.
struct RowIdentity<'a> {
    name: &'a str,
    school_name: &'a str,
    grade: Grade,
    gender: Gender,
    an_athlete_id: Option<u64>,
    timer_team_id: Option<u64>,
    an_team_id: Option<u64>,
}

/// The athlete and team one row mints, before its performance is written.
struct Mapped {
    athlete: AthleteId,
    team: TeamId,
}

/// Record one event document's row: its athlete, its team and the performance it published.
///
/// Returns true when the row yielded identity evidence, whether or not it published a mark.
pub(super) fn record_row(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    row: &DocRow,
    row_index: usize,
) -> bool {
    writer.stats.rows_read = writer.stats.rows_read.saturating_add(1);
    let Some(identity) = decode_row(writer, row) else {
        return false;
    };
    let Some(school_id) = resolve_school(writer, context, identity.school_name) else {
        return false;
    };
    writer.stats.rows_mapped = writer.stats.rows_mapped.saturating_add(1);
    let mapped = map_identity(writer, context, &school_id, &identity, row_index);
    count_channels(writer.stats, row);
    let Some(mark) = row.canonical_mark(context.kind) else {
        // `NH` publishes `im: 0`: the athlete competed, so the identity above is the evidence, and
        // there is no performance to write.
        writer.stats.rows_without_mark = writer.stats.rows_without_mark.saturating_add(1);
        return true;
    };
    let facts = PerformanceFacts {
        mark,
        wind_mps: row.wind_mps(),
        place: row.place(),
        heat: row.heat_number().map(|heat| heat.to_string()),
        grade: identity.grade,
        row: row_index,
    };
    write_performance(writer, context, &mapped, &facts);
    true
}

/// Record one live-standings row, whose mark arrives through the raw timing channel.
///
/// Returns true when the row yielded identity evidence, whether or not it published a mark.
pub(super) fn record_standing(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    row: &StandingRow,
    row_index: usize,
) -> bool {
    writer.stats.rows_read = writer.stats.rows_read.saturating_add(1);
    let Some(identity) = decode_standing(writer, row) else {
        return false;
    };
    let Some(school_id) = resolve_school(writer, context, identity.school_name) else {
        return false;
    };
    writer.stats.rows_mapped = writer.stats.rows_mapped.saturating_add(1);
    let mapped = map_identity(writer, context, &school_id, &identity, row_index);
    count_standing_channels(writer.stats, row);
    let Some(mark) = row.canonical_mark(context.kind) else {
        writer.stats.rows_without_mark = writer.stats.rows_without_mark.saturating_add(1);
        return true;
    };
    let facts = PerformanceFacts {
        mark,
        // The standings payload publishes no wind channel.
        wind_mps: None,
        place: row.place(),
        heat: None,
        grade: identity.grade,
        row: row_index,
    };
    write_performance(writer, context, &mapped, &facts);
    true
}

/// Count one refusal and report that no entity was written.
fn refused<T>(stats: &mut ResultStats, refusal: Refusal) -> Option<T> {
    match refusal {
        Refusal::NoName => {
            stats.rows_skipped_no_name = stats.rows_skipped_no_name.saturating_add(1);
        }
        Refusal::NoSchool => {
            stats.rows_skipped_no_school = stats.rows_skipped_no_school.saturating_add(1);
        }
        Refusal::BelowHighSchool => {
            stats.rows_skipped_below_high_school =
                stats.rows_skipped_below_high_school.saturating_add(1);
        }
        Refusal::NoGrade => {
            stats.rows_skipped_no_grade = stats.rows_skipped_no_grade.saturating_add(1);
        }
    }
    None
}

/// Decode one event-document row into the identity facts a mapping needs, counting its refusal.
fn decode_row<'a>(writer: &mut Writer<'_>, row: &'a DocRow) -> Option<RowIdentity<'a>> {
    let Some(athlete) = row.athlete.as_ref() else {
        return refused(writer.stats, Refusal::NoName);
    };
    let Some(name) = athlete
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    else {
        return refused(writer.stats, Refusal::NoName);
    };
    let Some(school_name) = athlete.team.as_ref().and_then(DocTeam::school_name) else {
        return refused(writer.stats, Refusal::NoSchool);
    };
    let grade = read_grade(writer.stats, athlete.grade.as_ref())?;
    let team = athlete.team.as_ref();
    Some(RowIdentity {
        name,
        school_name,
        grade,
        gender: athlete
            .gender_token()
            .map(gender_from_token)
            .unwrap_or(Gender::Unknown),
        an_athlete_id: athlete.an_athlete_id.as_ref().and_then(value_u64),
        timer_team_id: team
            .and_then(|team| team.timer_team_id.as_ref())
            .and_then(value_u64),
        an_team_id: team
            .and_then(|team| team.an_team_id.as_ref())
            .and_then(value_u64),
    })
}

/// Decode one live-standings row into the identity facts a mapping needs, counting its refusal.
fn decode_standing<'a>(writer: &mut Writer<'_>, row: &'a StandingRow) -> Option<RowIdentity<'a>> {
    let Some(name) = row.name() else {
        return refused(writer.stats, Refusal::NoName);
    };
    let Some(school_name) = row.school_name() else {
        return refused(writer.stats, Refusal::NoSchool);
    };
    let grade = read_grade(writer.stats, row.grade.as_ref())?;
    Some(RowIdentity {
        name,
        school_name,
        grade,
        gender: row
            .gender
            .as_deref()
            .map(gender_from_token)
            .unwrap_or(Gender::Unknown),
        an_athlete_id: row.an_athlete_id(),
        // The standings payload publishes no team id, only the run's short team key, which is not a
        // team identity this census can hold: it is counted (`rows_with_timer_team_key`), never
        // minted.
        timer_team_id: None,
        an_team_id: None,
    })
}

/// Read a published grade token, counting the refusal that leaves no grade.
fn read_grade(stats: &mut ResultStats, value: Option<&Value>) -> Option<Grade> {
    let token = match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    };
    if token.is_empty() {
        return refused(stats, Refusal::NoGrade);
    }
    match grade_from_token(&token) {
        Some(grade) => Some(grade),
        // A numeric token outside 9..=12 is a below-high-school entry: an open meet publishes one,
        // and the census cohort holds only high-school grades.
        None if token.parse::<u8>().is_ok() => refused(stats, Refusal::BelowHighSchool),
        None => refused(stats, Refusal::NoGrade),
    }
}

/// Count the channels one event-document row publishes beside its mark.
///
/// These describe the rows that mapped, because a row refused for identity reasons never has its
/// mark or its heat read.
fn count_channels(stats: &mut ResultStats, row: &DocRow) {
    if row.heat_number().is_some() {
        stats.rows_with_heat = stats.rows_with_heat.saturating_add(1);
    }
    if row.wind_mps().is_some() {
        stats.rows_with_wind = stats.rows_with_wind.saturating_add(1);
    }
    if row
        .seed
        .as_deref()
        .is_some_and(|seed| !seed.trim().is_empty())
    {
        stats.rows_with_seed = stats.rows_with_seed.saturating_add(1);
    }
    if !row.splits.is_empty() {
        stats.rows_with_splits = stats.rows_with_splits.saturating_add(1);
        stats.splits = stats.splits.saturating_add(row.splits.len());
    }
    if row.place().is_none() {
        stats.rows_unplaced = stats.rows_unplaced.saturating_add(1);
    }
}

/// Count the channels one live-standings row publishes beside its mark.
fn count_standing_channels(stats: &mut ResultStats, row: &StandingRow) {
    if row.has_legacy_id() {
        stats.rows_with_legacy_id = stats.rows_with_legacy_id.saturating_add(1);
    }
    if row.has_team_key() {
        stats.rows_with_timer_team_key = stats.rows_with_timer_team_key.saturating_add(1);
    }
    let splits = row.split_count();
    if splits > 0 {
        stats.rows_with_splits = stats.rows_with_splits.saturating_add(1);
        stats.splits = stats.splits.saturating_add(splits);
    }
    if row.place().is_none() {
        stats.rows_unplaced = stats.rows_unplaced.saturating_add(1);
    }
}
