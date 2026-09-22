//! One mapped `/raw` row: the school it resolves to, the athlete it observes, and the performance it
//! mints.
//!
//! Grade handling is the point of this route, and it is deliberately asymmetric. The file publishes
//! `Yr` — the athlete's grade *at the meet* — not a graduating year, and the row itself carries no
//! school year, so the grade is kept as evidence: an `ObservedGrade` record (grade + the school year
//! the meet's published date sits in + the source) travels on the athlete, and the canonical grad
//! year is the projection those two fields produce. The published cell is quoted in the
//! performance's evidence note, so a reviewer can check the projection against the row it came from
//! without re-fetching the file.
//!
//! A row with no `Yr` cannot be projected onto a grad year at all — the identity the census mints
//! athletes on — so such a row is counted and contributes no entity, exactly as the roster route
//! drops a roster row whose `column-grad-year` cell does not parse.

use super::{MeetContext, RowWriter};
use crate::school_index::SchoolIndex;
use crate::sources::hytek;
use crate::sources::result_file::ParsedRow;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, CanonicalTeam, Evidence, Gender, GradYear,
    Grade, ObservedGrade, SchoolId, SchoolYear, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

/// Count one result row and write the athlete and performance it names.
pub(super) fn record_row(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
) -> usize {
    bump(&mut writer.stats.rows);
    let Some(shape) = shape_of(writer, context, row) else {
        return 0;
    };
    let Some(school_id) = resolve_school(writer, context, row) else {
        return 0;
    };
    let team_id = team_for(
        &mut writer.accumulated.teams,
        &school_id,
        shape.sport,
        context.event.gender,
        context.school_year,
        context.evidence,
    );
    let athlete_id = record_athlete(writer, context, &school_id, &row.name, shape.grade);
    record_performance(
        writer,
        context,
        row,
        row_index,
        shape,
        &athlete_id,
        &team_id,
    );
    1
}

/// The two facts a row must publish before it can be minted at all: the sport that places it under a
/// team, and the grade that projects onto a graduation year.
struct RowShape {
    sport: Sport,
    grade: Grade,
}

/// The guards a row must pass, each one counted under its own name so a run reports why rows vanish
/// rather than dropping them silently.
///
/// A page that names no sport leaves its rows without a team or a performance to hang on; a row with
/// no `Yr` has no grad-year projection; a row whose athlete cell repeats its own school's name is the
/// shape a relay row takes in a file that lists the school where the athlete belongs — no individual
/// row can legitimately do that, and the guard can only fire when the two cells name the same school,
/// so it cannot drop a real athlete. (The file's relay shape is itself unverified: no relay result
/// set was captured.)
fn shape_of(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
) -> Option<RowShape> {
    let Some(sport) = context.sport else {
        bump(&mut writer.stats.rows_without_sport);
        return None;
    };
    let Some(grade) = row.grade else {
        bump(&mut writer.stats.rows_without_grade);
        return None;
    };
    bump(&mut writer.stats.rows_with_grade);
    if row.school.trim().is_empty() {
        bump(&mut writer.stats.rows_without_school);
        return None;
    }
    if !hytek::looks_like_a_name(&row.name) {
        bump(&mut writer.stats.rows_without_name);
        return None;
    }
    if census_domain::model::normalize_name(&row.name)
        == census_domain::model::normalize_name(&row.school)
    {
        bump(&mut writer.stats.rows_school_named);
        return None;
    }
    Some(RowShape { sport, grade })
}

/// Write one row's performance, minted on the same key the shared pass writes so a re-run lands on one
/// entity.
fn record_performance(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
    shape: RowShape,
    athlete_id: &AthleteId,
    team_id: &TeamId,
) {
    let source_key = performance_key(context, row_index);
    let performance_id = CanonicalPerformance::mint(
        athlete_id,
        &context.meet.id,
        &context.event.kind,
        &context.meet.date,
        &source_key,
    );
    let evidence = performance_evidence(context, shape.grade, row_index);
    writer
        .accumulated
        .performances
        .entry(performance_id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id: performance_id,
            athlete: athlete_id.clone(),
            team: team_id.clone(),
            event: context.event_id.clone(),
            meet: context.meet.id.clone(),
            date: context.meet.date.clone(),
            mark: row.mark.clone(),
            wind_mps: row.wind_mps,
            place: row.place,
            heat: row.heat.clone(),
            round: context.event.round.clone(),
            // The `/raw` payload states no timing method, and MileSplit is not the timer; nothing on
            // the page lets one be read, so none is claimed.
            timing: Some(TimingMethod::Unknown),
            observed_grade: Some(shape.grade),
            evidence: vec![evidence],
            source_key,
        });
}

/// Count one dropped row under its own name.
fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}

/// Resolve a row's published school label inside the site's own jurisdiction, memoising hits and
/// misses alike.
fn resolve_school(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
) -> Option<SchoolId> {
    let index: &SchoolIndex = writer.index;
    let jurisdiction: UsJurisdiction = context.jurisdiction;
    writer
        .resolved
        .entry(row.school.clone())
        .or_insert_with(|| match index.resolve(jurisdiction, &row.school) {
            Some((id, kind)) => {
                let slot = writer
                    .stats
                    .school_resolved
                    .entry(kind.as_str())
                    .or_default();
                *slot = slot.saturating_add(1);
                Some(id)
            }
            None => {
                let slot = writer
                    .stats
                    .unresolved
                    .entry(row.school.clone())
                    .or_default();
                *slot = slot.saturating_add(1);
                None
            }
        })
        .clone()
}

/// Record one row's athlete: their sports, the published grade as evidence, and the meet's evidence.
fn record_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    school_id: &SchoolId,
    member_name: &str,
    grade: Grade,
) -> AthleteId {
    let grad_year = GradYear::of(grade, context.school_year);
    let athlete_id =
        CanonicalAthlete::mint(school_id, member_name, grad_year, context.event.gender);
    let entry = writer
        .accumulated
        .athletes
        .entry(athlete_id.as_str().to_string())
        .or_insert_with(|| {
            let mut athlete =
                CanonicalAthlete::new(school_id, member_name, grad_year, context.event.gender);
            if let Some(sport) = context.sport {
                athlete.sports.push(sport);
            }
            athlete
        });
    if let Some(sport) = context.sport {
        if !entry.sports.contains(&sport) {
            entry.sports.push(sport);
        }
    }
    // The grade the row published, dated by the school year the meet sits in: the evidence the
    // canonical grad year above is a projection of.
    let observation = ObservedGrade {
        grade,
        school_year: context.school_year,
        source: context.source.clone(),
    };
    if !entry.observed_grades.contains(&observation) {
        entry.observed_grades.push(observation);
    }
    if !entry
        .evidence
        .iter()
        .any(|existing| existing == context.evidence)
    {
        entry.evidence.push(context.evidence.clone());
    }
    athlete_id
}

/// The deterministic per-row performance key: the result set (`RSID`), the section, and the row.
fn performance_key(context: &MeetContext<'_>, row_index: usize) -> String {
    format!("{}:{}:{}", context.rsid, context.event.label, row_index)
}

/// The performance's evidence: the meet's, annotated with the `Yr` cell this row published.
fn performance_evidence(context: &MeetContext<'_>, grade: Grade, row_index: usize) -> Evidence {
    let mut evidence = context.evidence.clone();
    evidence.note = Some(format!(
        "RSID {} row {row_index}: Yr {} published on the row, read as grade evidence for school \
         year {}",
        context.rsid,
        grade.get(),
        context.school_year.start_year()
    ));
    evidence
}

/// The team a row belongs to: one per (school, sport, gender, school year), minted on the same key
/// the roster route uses so both routes land on one team.
fn team_for(
    teams: &mut HashMap<String, CanonicalTeam>,
    school: &SchoolId,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    evidence: &Evidence,
) -> TeamId {
    let key = format!(
        "{}:{sport:?}:{gender:?}:{}",
        school.as_str(),
        school_year.start_year()
    );
    teams
        .entry(key)
        .or_insert_with(|| {
            let id = CanonicalTeam::mint(school, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
            }
        })
        .id
        .clone()
}
