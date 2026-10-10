use super::super::read::{grade_of, meet_mark};
use super::super::store::{athlete, store, AthleteRow};
use super::super::wire::{FlatRow, PublishedLeg};
use super::{events::MetadataConflict, MeetCtx};
use crate::athleticnet::map::{school_for, PerformanceInput, SchoolResolveContext};
use crate::athleticnet::parse::timing_of;
use census_domain::model::{AthleteId, EventKind, Gender, Grade, Mark, SchoolId, SourceIdentity};
use std::collections::BTreeMap;

pub(super) struct Block<'a> {
    pub(super) kind: &'a EventKind,
    pub(super) gender: Gender,
    pub(super) labels: &'a [&'a str],
    pub(super) type_hint: Option<&'a str>,
    pub(super) division: Option<String>,
    pub(super) round: Option<String>,
    pub(super) metadata_conflict: Option<MetadataConflict<'a>>,
}

pub(super) struct Entry<'a> {
    pub(super) school: &'a SchoolId,
    pub(super) athlete: &'a AthleteId,
    pub(super) source_athlete: SourceIdentity,
    pub(super) mark: Mark,
    pub(super) auto: bool,
    pub(super) place: Option<&'a str>,
    pub(super) grade: Grade,
    pub(super) source_key: String,
    pub(super) leg: Option<usize>,
    pub(super) published: &'a str,
}

impl MeetCtx<'_> {
    pub(super) fn individual_row(
        &mut self,
        block: &Block<'_>,
        row: &FlatRow,
    ) -> crate::CrawlResult<()> {
        let Some(school) = self.school(row) else {
            return Ok(());
        };
        let Some(provider_id) = row.athlete_id else {
            self.counts.rows_no_athlete = self.counts.rows_no_athlete.saturating_add(1);
            return Ok(());
        };
        let Some(grade) = grade_of(row.grade.as_deref()) else {
            self.counts.rows_no_grade = self.counts.rows_no_grade.saturating_add(1);
            return Ok(());
        };
        let Some(name) = row.name() else {
            self.counts.rows_no_name = self.counts.rows_no_name.saturating_add(1);
            return Ok(());
        };
        let (mark, auto) = self.mark_of(block, &row.result);
        let source_key = format!("athleticnet:{provider_id}-{}", row.result_id);
        let Some((athlete_id, source_athlete)) = self.athlete_of(
            provider_id,
            &school,
            &name,
            grade,
            block.gender,
            &source_key,
        ) else {
            self.counts.rows_unsupported_cohort =
                self.counts.rows_unsupported_cohort.saturating_add(1);
            return Ok(());
        };
        let entry = Entry {
            school: &school,
            athlete: &athlete_id,
            source_athlete,
            mark,
            auto,
            place: row.place.as_deref(),
            grade,
            source_key,
            leg: None,
            published: &row.result,
        };
        self.store_row(block, entry)?;
        self.counts.rows_stored = self.counts.rows_stored.saturating_add(1);
        Ok(())
    }

    pub(super) fn relay_row(
        &mut self,
        block: &Block<'_>,
        row: &FlatRow,
        legs: &BTreeMap<i64, Vec<&PublishedLeg>>,
    ) -> crate::CrawlResult<()> {
        let Some(school) = self.school(row) else {
            return Ok(());
        };
        let Some(squad) = legs.get(&row.result_id) else {
            self.counts.relay_rows_without_legs =
                self.counts.relay_rows_without_legs.saturating_add(1);
            return Ok(());
        };
        self.counts.relay_rows = self.counts.relay_rows.saturating_add(1);
        self.counts.legs_seen = self
            .counts
            .legs_seen
            .saturating_add(u64::try_from(squad.len()).map_or(u64::MAX, |value| value));
        let (mark, auto) = self.mark_of(block, &row.result);
        let mut outcome = Ok(());
        for (index, leg) in squad.iter().enumerate() {
            let projected = self.relay_leg(
                block,
                row,
                leg,
                index.saturating_add(1),
                &school,
                (&mark, auto),
            );
            outcome = outcome.and(projected);
        }
        outcome
    }

    fn relay_leg(
        &mut self,
        block: &Block<'_>,
        row: &FlatRow,
        leg: &PublishedLeg,
        position: usize,
        school: &SchoolId,
        result: (&Mark, bool),
    ) -> crate::CrawlResult<()> {
        let Some(provider_id) = leg.athlete_id else {
            self.counts.legs_no_athlete = self.counts.legs_no_athlete.saturating_add(1);
            return Ok(());
        };
        let name = leg.name.trim();
        if name.is_empty() {
            self.counts.legs_no_name = self.counts.legs_no_name.saturating_add(1);
            return Ok(());
        }
        let Some(grade) = grade_of(leg.short_desc.as_deref()) else {
            self.counts.legs_no_grade = self.counts.legs_no_grade.saturating_add(1);
            return Ok(());
        };
        let source_key = format!("athleticnet:{provider_id}-{}:leg{position}", row.result_id);
        let Some((athlete_id, source_athlete)) =
            self.athlete_of(provider_id, school, name, grade, block.gender, &source_key)
        else {
            self.counts.legs_unsupported_cohort =
                self.counts.legs_unsupported_cohort.saturating_add(1);
            return Ok(());
        };
        let entry = Entry {
            school,
            athlete: &athlete_id,
            source_athlete,
            mark: result.0.clone(),
            auto: result.1,
            place: row.place.as_deref(),
            grade,
            source_key,
            leg: Some(position),
            published: &row.result,
        };
        self.store_row(block, entry)?;
        self.counts.legs_stored = self.counts.legs_stored.saturating_add(1);
        Ok(())
    }

    fn school(&mut self, row: &FlatRow) -> Option<SchoolId> {
        let Some(team_id) = row.team_id else {
            self.stats.rows_unknown_school = self.stats.rows_unknown_school.saturating_add(1);
            return None;
        };
        let mut context = SchoolResolveContext {
            state: Some(self.state),
            school_names: &self.school_names,
            index: self.index,
            resolved: self.resolved,
            source: self.source,
            observed_on: self.observed_on,
            stats: self.stats,
            accumulated: self.accumulated,
        };
        school_for(&team_id.to_string(), &mut context)
    }

    fn athlete_of(
        &mut self,
        provider_id: i64,
        school: &SchoolId,
        name: &str,
        grade: Grade,
        gender: Gender,
        source_row: &str,
    ) -> Option<(AthleteId, SourceIdentity)> {
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
                source_row,
            },
        )
    }

    fn store_row(&mut self, block: &Block<'_>, entry: Entry<'_>) -> crate::CrawlResult<()> {
        let input = PerformanceInput {
            athlete: entry.athlete,
            source_athlete: entry.source_athlete,
            school: entry.school,
            meet: &self.meet,
            kind: block.kind,
            sport: self.sport,
            gender: block.gender,
            school_year: self.school_year,
            performance_as_of: self.performance_as_of,
            grade: Some(entry.grade),
            date: self.date.clone(),
            mark: entry.mark,
            wind_mps: None,
            place: entry.place,
            round: block.round.clone(),
            timing: timing_of(0, entry.auto),
            division: block.division.clone(),
            source_key: entry.source_key,
            labels: block.labels,
        };
        let note = entry.leg.map(|position| {
            format!(
                "relay leg {position}; the mark and place published are the relay squad's, which \
                 the payload does not split per leg"
            )
        });
        store(
            self.accumulated,
            self.source,
            self.observed_on,
            input,
            note,
            entry.published,
            block.metadata_conflict,
        )
    }

    fn mark_of(&mut self, block: &Block<'_>, published: &str) -> (Mark, bool) {
        if matches!(block.kind, EventKind::Unmapped { .. }) && block.type_hint.is_none() {
            self.counts.rows_unmapped_event = self.counts.rows_unmapped_event.saturating_add(1);
        }
        let parsed = meet_mark(block.kind, published, block.type_hint);
        if parsed
            .as_ref()
            .is_none_or(|(mark, _)| matches!(mark, Mark::Raw(_)))
        {
            self.counts.rows_no_mark = self.counts.rows_no_mark.saturating_add(1);
        }
        match parsed {
            Some(mark) => mark,
            None => (Mark::Raw(published.to_string()), false),
        }
    }
}
