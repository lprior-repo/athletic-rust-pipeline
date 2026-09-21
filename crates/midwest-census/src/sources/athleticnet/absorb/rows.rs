//! The row walk: one payload's track and cross-country rows, resolved to the identifiers the
//! performance store needs, then stored or counted as refused.

use super::Ctx;
use crate::sources::athleticnet::map::{grade_in, meet_for, store_performance, PerformanceInput};
use crate::sources::athleticnet::parse::{parse_mark, round_of, timing_of, Bio, TfRow, XcRow};
use census_domain::model::{
    AthleteId, CanonicalMeet, EventKind, Gender, Grade, Mark, SchoolId, SchoolYear, Sport,
    TimingMethod,
};
use std::collections::HashMap;

/// One row, resolved to the identifiers the performance store needs.
struct ResolvedRow<'a> {
    kind: EventKind,
    sport: Sport,
    meet: CanonicalMeet,
    date: String,
    school: SchoolId,
    school_year: SchoolYear,
    grade: Option<Grade>,
    mark: Mark,
    timing: Option<TimingMethod>,
    label: Option<&'a str>,
}

impl<'a> Ctx<'a> {
    /// Walk one payload's track rows, returning how many performances were stored.
    pub(super) fn track_rows(&mut self, bio: &Bio, athlete_id: &AthleteId, gender: Gender) -> u64 {
        let labels: HashMap<i64, &str> = bio
            .events
            .iter()
            .flatten()
            .map(|event| (event.id, event.label.as_str()))
            .collect();
        let mut rows = 0u64;
        for row in bio.results_tf.iter().flatten() {
            if let Some(resolved) = self.resolve_tf_row(bio, row, &labels) {
                self.store_tf_row(row, &resolved, athlete_id, gender);
                rows += 1;
            }
        }
        rows
    }

    /// Walk one payload's cross-country rows, returning how many performances were stored.
    pub(super) fn cross_rows(&mut self, bio: &Bio, athlete_id: &AthleteId, gender: Gender) -> u64 {
        let mut rows = 0u64;
        for row in bio.results_xc.iter().flatten() {
            if let Some(resolved) = self.resolve_xc_row(bio, row) {
                self.store_xc_row(row, &resolved, athlete_id, gender);
                rows += 1;
            }
        }
        rows
    }

    /// Resolve one track row, counting why a row is refused; `None` when it stores nothing.
    fn resolve_tf_row<'b>(
        &mut self,
        bio: &'b Bio,
        row: &'b TfRow,
        labels: &'b HashMap<i64, &'b str>,
    ) -> Option<ResolvedRow<'b>> {
        self.stats.rows_seen += 1;
        let (season_id, sport) = self.row_season(row)?;
        let Some(label) = row.event_id.and_then(|id| labels.get(&id).copied()) else {
            self.stats.rows_no_event += 1;
            return None;
        };
        let kind = EventKind::from_source_label(label);
        let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
            self.stats.rows_no_mark += 1;
            return None;
        };
        let Some(meet) = meet_for(
            row.meet_id,
            bio,
            self.target.state.as_deref(),
            self.source,
            self.observed_on,
            self.accumulated,
        ) else {
            self.stats.rows_unknown_meet += 1;
            return None;
        };
        let Some(date) = row.date().or_else(|| Some(meet.date.clone())) else {
            self.stats.rows_unknown_meet += 1;
            return None;
        };
        let school = self.canonical_school(&row.school_id.unwrap_or_default().to_string())?;
        let school_year = SchoolYear::containing(season_id, 5);
        Some(ResolvedRow {
            kind,
            sport,
            meet,
            date,
            school,
            school_year,
            grade: grade_in(&self.observed_grades, school_year),
            mark,
            timing: timing_of(row.fat, auto),
            label: Some(label),
        })
    }

    /// Resolve one cross-country row, counting why a row is refused; `None` when it stores nothing.
    fn resolve_xc_row<'b>(&mut self, bio: &'b Bio, row: &'b XcRow) -> Option<ResolvedRow<'b>> {
        self.stats.rows_seen += 1;
        let Some(season_id) = row.season_id else {
            self.stats.rows_no_season += 1;
            return None;
        };
        let kind = EventKind::CrossCountry;
        let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
            self.stats.rows_no_mark += 1;
            return None;
        };
        let Some(meet) = meet_for(
            row.meet_id,
            bio,
            self.target.state.as_deref(),
            self.source,
            self.observed_on,
            self.accumulated,
        ) else {
            self.stats.rows_unknown_meet += 1;
            return None;
        };
        let date = meet.date.clone();
        let school = self.canonical_school(&row.school_id.unwrap_or_default().to_string())?;
        // A cross-country season is a fall season, so its school year starts in the same
        // calendar year the season is named for.
        let school_year = SchoolYear::containing(season_id, 9);
        Some(ResolvedRow {
            kind,
            sport: Sport::CrossCountry,
            meet,
            date,
            school,
            school_year,
            grade: grade_in(&self.observed_grades, school_year),
            mark,
            timing: timing_of(0, auto),
            label: None,
        })
    }

    /// The season a track row publishes and its indoor/outdoor split, counting a season the
    /// payload omits.
    fn row_season(&mut self, row: &TfRow) -> Option<(i16, Sport)> {
        let Some(season_id) = row.season_id else {
            self.stats.rows_no_season += 1;
            return None;
        };
        let Some(sport) = self
            .seasons
            .get(&(row.school_id.unwrap_or_default(), season_id))
            .copied()
            .flatten()
        else {
            *self
                .stats
                .rows_unknown_season
                .entry(season_id.to_string())
                .or_default() += 1;
            return None;
        };
        Some((season_id, sport))
    }

    /// Store one resolved track row's performance and count it absorbed.
    fn store_tf_row(
        &mut self,
        row: &TfRow,
        resolved: &ResolvedRow<'_>,
        athlete_id: &AthleteId,
        gender: Gender,
    ) {
        let source_key = format!("athleticnet:{}-{}", self.target.athlete_id, row.id);
        store_performance(
            self.accumulated,
            self.source,
            self.observed_on,
            PerformanceInput {
                athlete: athlete_id,
                school: &resolved.school,
                meet: &resolved.meet,
                kind: &resolved.kind,
                sport: resolved.sport,
                gender,
                school_year: resolved.school_year,
                grade: resolved.grade,
                date: resolved.date.clone(),
                mark: resolved.mark.clone(),
                wind_mps: row.wind,
                place: row.place.as_deref(),
                round: round_of(row.round.as_deref()),
                timing: resolved.timing,
                division: row.division.clone(),
                source_key,
                label: resolved.label,
            },
        );
        self.stats.rows_absorbed += 1;
    }

    /// Store one resolved cross-country row's performance and count it absorbed.
    fn store_xc_row(
        &mut self,
        row: &XcRow,
        resolved: &ResolvedRow<'_>,
        athlete_id: &AthleteId,
        gender: Gender,
    ) {
        let source_key = format!("athleticnet:{}-{}", self.target.athlete_id, row.id);
        store_performance(
            self.accumulated,
            self.source,
            self.observed_on,
            PerformanceInput {
                athlete: athlete_id,
                school: &resolved.school,
                meet: &resolved.meet,
                kind: &resolved.kind,
                sport: Sport::CrossCountry,
                gender,
                school_year: resolved.school_year,
                grade: resolved.grade,
                date: resolved.date.clone(),
                mark: resolved.mark.clone(),
                wind_mps: None,
                place: row.place.as_deref(),
                round: None,
                timing: resolved.timing,
                division: row
                    .division
                    .clone()
                    .or_else(|| row.distance.map(|metres| format!("{metres}m"))),
                source_key,
                label: None,
            },
        );
        self.stats.rows_absorbed += 1;
    }
}
