//! The rows of one block: the individual results, the relay squads and their legs, and the mints
//! each row needs before it can be stored.

use super::super::read::{grade_of, meet_mark};
use super::super::store::{athlete, store, AthleteRow};
use super::super::wire::{FlatRow, PublishedLeg};
use super::MeetCtx;
use crate::athleticnet::map::{school_for, PerformanceInput};
use crate::athleticnet::parse::timing_of;
use census_domain::model::{AthleteId, EventKind, Gender, Grade, Mark, SchoolId};
use std::collections::BTreeMap;

/// What every row of one results block shares.
pub(super) struct Block<'a> {
    pub(super) kind: &'a EventKind,
    pub(super) gender: Gender,
    /// The label the canonical event carries.
    pub(super) label: &'a str,
    /// The event type (`"T"`/`"F"`) the metadata document declares, when it was spent.
    pub(super) type_hint: Option<&'a str>,
    /// The division the block publishes (or the payload's division table resolves).
    pub(super) division: Option<String>,
    /// The round the block publishes, normalised.
    pub(super) round: Option<String>,
}

/// What one performance row contributes, once its identities are resolved.
pub(super) struct Entry<'a> {
    pub(super) school: &'a SchoolId,
    pub(super) athlete: &'a AthleteId,
    pub(super) mark: Mark,
    pub(super) auto: bool,
    pub(super) place: Option<&'a str>,
    pub(super) grade: Grade,
    /// The result identity, in the form the bio path mints for the same published result.
    pub(super) source_key: String,
    /// The leg's 1-based position in its relay, when the row is a relay leg.
    pub(super) leg: Option<usize>,
}

impl MeetCtx<'_> {
    /// One individual row, in the order the walk refuses it: school, athlete id, grade, name, mark.
    pub(super) fn individual_row(&mut self, block: &Block<'_>, row: &FlatRow) {
        let Some(school) = self.school(row) else {
            return;
        };
        let Some(provider_id) = row.athlete_id else {
            self.counts.rows_no_athlete = self.counts.rows_no_athlete.saturating_add(1);
            return;
        };
        let Some(grade) = grade_of(row.grade.as_deref()) else {
            self.counts.rows_no_grade = self.counts.rows_no_grade.saturating_add(1);
            return;
        };
        let Some(name) = row.name() else {
            self.counts.rows_no_name = self.counts.rows_no_name.saturating_add(1);
            return;
        };
        if self.refuse_unmapped_label(block) {
            return;
        }
        let Some((mark, auto)) = meet_mark(block.kind, &row.result, block.type_hint) else {
            self.counts.rows_no_mark = self.counts.rows_no_mark.saturating_add(1);
            return;
        };
        let athlete_id = self.athlete_of(provider_id, &school, &name, grade, block.gender);
        let entry = Entry {
            school: &school,
            athlete: &athlete_id,
            mark,
            auto,
            place: row.place.as_deref(),
            grade,
            source_key: format!("athleticnet:{provider_id}-{}", row.result_id),
            leg: None,
        };
        self.store_row(block, entry);
        self.counts.rows_stored = self.counts.rows_stored.saturating_add(1);
    }

    /// One relay squad row: the squad's mark and place belong to every leg it publishes, and the
    /// squad's own padded name is never attributed to a person.
    pub(super) fn relay_row(
        &mut self,
        block: &Block<'_>,
        row: &FlatRow,
        legs: &BTreeMap<i64, Vec<&PublishedLeg>>,
    ) {
        let Some(school) = self.school(row) else {
            return;
        };
        let Some(squad) = legs.get(&row.result_id) else {
            self.counts.relay_rows_without_legs =
                self.counts.relay_rows_without_legs.saturating_add(1);
            return;
        };
        self.counts.relay_rows = self.counts.relay_rows.saturating_add(1);
        self.counts.legs_seen = self
            .counts
            .legs_seen
            .saturating_add(u64::try_from(squad.len()).unwrap_or(u64::MAX));
        if self.refuse_unmapped_label(block) {
            return;
        }
        let Some((mark, auto)) = meet_mark(block.kind, &row.result, block.type_hint) else {
            self.counts.rows_no_mark = self.counts.rows_no_mark.saturating_add(1);
            return;
        };
        for (index, leg) in squad.iter().enumerate() {
            self.relay_leg(
                block,
                row,
                leg,
                index.saturating_add(1),
                &school,
                &mark,
                auto,
            );
        }
    }

    /// One relay leg of a squad whose mark this walk already read: the squad's mark and place are
    /// the leg's, the leg's own identity is its athlete id and its 1-based position.
    #[allow(clippy::too_many_arguments)]
    fn relay_leg(
        &mut self,
        block: &Block<'_>,
        row: &FlatRow,
        leg: &PublishedLeg,
        position: usize,
        school: &SchoolId,
        mark: &Mark,
        auto: bool,
    ) {
        let Some(provider_id) = leg.athlete_id else {
            self.counts.legs_no_athlete = self.counts.legs_no_athlete.saturating_add(1);
            return;
        };
        let name = leg.name.trim();
        if name.is_empty() {
            self.counts.legs_no_name = self.counts.legs_no_name.saturating_add(1);
            return;
        }
        let Some(grade) = grade_of(leg.short_desc.as_deref()) else {
            self.counts.legs_no_grade = self.counts.legs_no_grade.saturating_add(1);
            return;
        };
        let athlete_id = self.athlete_of(provider_id, school, name, grade, block.gender);
        let entry = Entry {
            school,
            athlete: &athlete_id,
            mark: mark.clone(),
            auto,
            place: row.place.as_deref(),
            grade,
            source_key: format!("athleticnet:{provider_id}-{}:leg{position}", row.result_id),
            leg: Some(position),
        };
        self.store_row(block, entry);
        self.counts.legs_stored = self.counts.legs_stored.saturating_add(1);
    }

    /// The canonical school a row names. A row whose team id the payload's team list does not name
    /// is counted by `school_for` itself, exactly as the bio path counts it.
    fn school(&mut self, row: &FlatRow) -> Option<SchoolId> {
        let Some(team_id) = row.team_id else {
            self.stats.rows_unknown_school = self.stats.rows_unknown_school.saturating_add(1);
            return None;
        };
        school_for(
            &team_id.to_string(),
            Some(self.state),
            &self.school_names,
            self.index,
            self.resolved,
            self.source,
            self.observed_on,
            self.stats,
            self.accumulated,
        )
    }

    /// The athlete a row names, minted once per (athlete id, school).
    fn athlete_of(
        &mut self,
        provider_id: i64,
        school: &SchoolId,
        name: &str,
        grade: Grade,
        gender: Gender,
    ) -> AthleteId {
        athlete(
            self.accumulated,
            self.source,
            self.observed_on,
            AthleteRow {
                provider_id,
                school,
                name,
                grade,
                gender,
                school_year: self.school_year,
                sport: self.sport,
            },
        )
    }

    /// Store one performance against the event its own division and round belong to.
    fn store_row(&mut self, block: &Block<'_>, entry: Entry<'_>) {
        let input = PerformanceInput {
            athlete: entry.athlete,
            school: entry.school,
            meet: &self.meet,
            kind: block.kind,
            sport: self.sport,
            gender: block.gender,
            school_year: self.school_year,
            grade: Some(entry.grade),
            date: self.date.clone(),
            mark: entry.mark,
            wind_mps: None,
            place: entry.place,
            round: block.round.clone(),
            timing: timing_of(0, entry.auto),
            division: block.division.clone(),
            source_key: entry.source_key,
            label: Some(block.label),
        };
        let note = entry.leg.map(|position| {
            format!(
                "relay leg {position}; the mark and place published are the relay squad's, which \
                 the payload does not split per leg"
            )
        });
        store(self.accumulated, self.source, self.observed_on, input, note);
    }
}
