use super::{Ctx, ProfileRows, Scope};
use crate::athleticnet::map::{grade_in, meet_for};
use crate::athleticnet::parse::{parse_mark, timing_of, Bio, TfRow, XcRow};
use census_domain::model::{
    AthleteId, CanonicalMeet, EventKind, Gender, Grade, Mark, SchoolId, SchoolYear, Sport,
    TimingMethod,
};
use std::collections::HashMap;

mod projection;

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
    pub(super) fn track_rows(&mut self, bio: &Bio, athlete_id: &AthleteId, gender: Gender) -> u64 {
        let labels: HashMap<i64, &str> = bio
            .events
            .iter()
            .flatten()
            .map(|event| (event.id, event.label.as_str()))
            .collect();
        let mut rows = 0u64;
        for (position, row) in bio.results_tf.iter().flatten().enumerate() {
            if let Some(resolved) = self.resolve_tf_row(bio, row, &labels) {
                match self.store_tf_row(row, &resolved, athlete_id, gender) {
                    Ok(()) => rows = rows.saturating_add(1),
                    Err(error) => ProfileRows {
                        bio,
                        scope: Scope::TrackField,
                        source: self.source,
                        observed_on: self.observed_on,
                    }
                    .review(
                        self.accumulated,
                        &format!("resultsTF/{position}"),
                        &error.to_string(),
                    ),
                }
            } else {
                ProfileRows {
                    bio,
                    scope: Scope::TrackField,
                    source: self.source,
                    observed_on: self.observed_on,
                }
                .reject_result(
                    self.accumulated,
                    position,
                    row.id,
                    &row.result,
                    row.school_id,
                );
            }
        }
        rows
    }

    pub(super) fn cross_rows(&mut self, bio: &Bio, athlete_id: &AthleteId, gender: Gender) -> u64 {
        let mut rows = 0u64;
        for (position, row) in bio.results_xc.iter().flatten().enumerate() {
            if let Some(resolved) = self.resolve_xc_row(bio, row) {
                match self.store_xc_row(row, &resolved, athlete_id, gender) {
                    Ok(()) => rows = rows.saturating_add(1),
                    Err(error) => ProfileRows {
                        bio,
                        scope: Scope::CrossCountry,
                        source: self.source,
                        observed_on: self.observed_on,
                    }
                    .review(
                        self.accumulated,
                        &format!("resultsXC/{position}"),
                        &error.to_string(),
                    ),
                }
            } else {
                ProfileRows {
                    bio,
                    scope: Scope::CrossCountry,
                    source: self.source,
                    observed_on: self.observed_on,
                }
                .reject_result(
                    self.accumulated,
                    position,
                    row.id,
                    &row.result,
                    row.school_id,
                );
            }
        }
        rows
    }

    fn resolve_tf_row<'b>(
        &mut self,
        bio: &'b Bio,
        row: &'b TfRow,
        labels: &'b HashMap<i64, &'b str>,
    ) -> Option<ResolvedRow<'b>> {
        self.stats.rows_seen = self.stats.rows_seen.saturating_add(1);
        let (season_id, sport) = self.row_season(row)?;
        let Some(label) = row.event_id.and_then(|id| labels.get(&id).copied()) else {
            self.stats.rows_no_event = self.stats.rows_no_event.saturating_add(1);
            return None;
        };
        let kind = EventKind::from_source_label(label);
        let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
            self.stats.rows_no_mark = self.stats.rows_no_mark.saturating_add(1);
            return None;
        };
        let Some(meet) = meet_for(
            row.meet_id,
            bio,
            self.target.state,
            self.source,
            self.observed_on,
            self.accumulated,
        ) else {
            self.stats.rows_unknown_meet = self.stats.rows_unknown_meet.saturating_add(1);
            return None;
        };
        let Some(date) = row.date().or_else(|| Some(meet.date.clone())) else {
            self.stats.rows_unknown_meet = self.stats.rows_unknown_meet.saturating_add(1);
            return None;
        };
        let school = self.canonical_school(
            &row.school_id
                .map_or(Default::default(), core::convert::identity)
                .to_string(),
        )?;
        let Some(school_year) = SchoolYear::containing(season_id, 5) else {
            self.stats.rows_no_season = self.stats.rows_no_season.saturating_add(1);
            return None;
        };
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

    fn resolve_xc_row<'b>(&mut self, bio: &'b Bio, row: &'b XcRow) -> Option<ResolvedRow<'b>> {
        self.stats.rows_seen = self.stats.rows_seen.saturating_add(1);
        let Some(season_id) = row.season_id else {
            self.stats.rows_no_season = self.stats.rows_no_season.saturating_add(1);
            return None;
        };
        let kind = EventKind::CrossCountry;
        let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
            self.stats.rows_no_mark = self.stats.rows_no_mark.saturating_add(1);
            return None;
        };
        let Some(meet) = meet_for(
            row.meet_id,
            bio,
            self.target.state,
            self.source,
            self.observed_on,
            self.accumulated,
        ) else {
            self.stats.rows_unknown_meet = self.stats.rows_unknown_meet.saturating_add(1);
            return None;
        };
        let date = meet.date.clone();
        let school = self.canonical_school(
            &row.school_id
                .map_or(Default::default(), core::convert::identity)
                .to_string(),
        )?;
        let Some(school_year) = SchoolYear::containing(season_id, 9) else {
            self.stats.rows_no_season = self.stats.rows_no_season.saturating_add(1);
            return None;
        };
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

    fn row_season(&mut self, row: &TfRow) -> Option<(i16, Sport)> {
        let Some(season_id) = row.season_id else {
            self.stats.rows_no_season = self.stats.rows_no_season.saturating_add(1);
            return None;
        };
        let Some(sport) = self
            .seasons
            .get(&(
                row.school_id
                    .map_or(Default::default(), core::convert::identity),
                season_id,
            ))
            .copied()
            .flatten()
        else {
            let season = self
                .stats
                .rows_unknown_season
                .entry(season_id.to_string())
                .or_default();
            *season = (*season).saturating_add(1);
            return None;
        };
        Some((season_id, sport))
    }
}
