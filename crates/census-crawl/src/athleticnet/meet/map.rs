pub(super) mod events;
mod rows;

use super::count::MeetStats;
use super::read::{
    event_type_agrees, jurisdiction_of, legs_by_result, meet_date, sport_of, EventMetadata,
};
use super::store::meet_row;
use super::wire::{AllResults, FlatEvent, MeetData, PublishedLeg};
use crate::athleticnet::map::{Accumulator, Stats};
use crate::athleticnet::parse::{gender_of, round_of};
use census_domain::model::{CanonicalMeet, EventKind, SchoolId, SchoolYear, SourceRef};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use rows::Block;
use std::collections::{BTreeMap, HashMap};

pub(in crate::athleticnet) fn absorb_meet(
    meet: &MeetData,
    results: &AllResults,
    metadata: Option<&EventMetadata>,
    source: &SourceRef,
    timing: (&str, chrono::NaiveDate),
    index: &SchoolIndex,
    resolved: &mut HashMap<String, SchoolId>,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> crate::CrawlResult<(u64, MeetStats)> {
    let mut counts = MeetStats::default();
    let (observed_on, performance_as_of) = timing;
    let date = meet_date(&meet.meet.date).map_or(meet.meet.date.as_str(), |value| value);
    if matches!(
        crate::context::assess_performance_date(performance_as_of, date),
        crate::context::PerformanceDateAssessment::Unknown
    ) {
        return Err(crate::CrawlError::PerformanceDateUnknown {
            published: meet.meet.date.chars().take(64).collect(),
            as_of: performance_as_of,
        });
    }
    let Some(place) = place_meet(meet, &mut counts) else {
        return Ok((0, counts));
    };
    let sport = sport_of(meet.sport2.as_deref());
    let divisions: HashMap<i64, &str> = meet
        .divisions
        .iter()
        .map(|division| (division.id, division.name.as_str()))
        .collect();
    let legs = legs_by_result(&results.legs);
    let row = meet_row(
        meet,
        place.state,
        &place.date,
        sport,
        source,
        observed_on,
        accumulated,
    );
    let mut ctx = MeetCtx {
        source,
        observed_on,
        performance_as_of,
        index,
        resolved,
        stats,
        accumulated,
        counts: &mut counts,
        metadata,
        state: place.state,
        sport,
        school_year: place.school_year,
        date: place.date,
        meet: row,
        school_names: school_names(results),
    };
    ctx.walk(results, &legs, &divisions)?;
    counts.meets_pulled = counts.meets_pulled.saturating_add(1);
    let stored = counts.stored();
    Ok((stored, counts))
}

fn meet_state(meet: &MeetData) -> Option<UsJurisdiction> {
    jurisdiction_of(
        meet.meet
            .location
            .as_ref()
            .and_then(|location| location.state.as_deref()),
    )
}

fn season_of(date: &str) -> Option<i16> {
    date.split('-').next()?.trim().parse::<i16>().ok()
}

struct Placement {
    state: UsJurisdiction,
    date: String,
    school_year: SchoolYear,
}

fn place_meet(meet: &MeetData, counts: &mut MeetStats) -> Option<Placement> {
    let Some(state) = meet_state(meet) else {
        counts.meets_unplaced = counts.meets_unplaced.saturating_add(1);
        return None;
    };
    let Some(date) = meet_date(&meet.meet.date) else {
        counts.meets_without_date = counts.meets_without_date.saturating_add(1);
        return None;
    };
    let date = date.to_string();
    let Some(season) = meet.meet.season_id.or_else(|| season_of(&date)) else {
        counts.meets_without_season = counts.meets_without_season.saturating_add(1);
        return None;
    };
    let Some(school_year) = SchoolYear::containing(season, 5) else {
        counts.meets_without_season = counts.meets_without_season.saturating_add(1);
        return None;
    };
    Some(Placement {
        state,
        date,
        school_year,
    })
}

fn school_names(results: &AllResults) -> HashMap<String, &str> {
    results
        .teams
        .iter()
        .map(|team| (team.school_id.to_string(), team.school_name.as_str()))
        .collect()
}

struct MeetCtx<'a> {
    source: &'a SourceRef,
    observed_on: &'a str,
    performance_as_of: chrono::NaiveDate,
    index: &'a SchoolIndex,
    resolved: &'a mut HashMap<String, SchoolId>,
    stats: &'a mut Stats,
    accumulated: &'a mut Accumulator,
    counts: &'a mut MeetStats,
    metadata: Option<&'a EventMetadata>,
    state: UsJurisdiction,
    sport: Option<census_domain::model::Sport>,
    school_year: SchoolYear,
    date: String,
    meet: CanonicalMeet,
    school_names: HashMap<String, &'a str>,
}

fn kind_of(event: &FlatEvent) -> EventKind {
    [&event.label, &event.short]
        .into_iter()
        .map(|label| EventKind::from_source_label(label))
        .find(|kind| !matches!(kind, EventKind::Unmapped { .. }))
        .map_or_else(
            || EventKind::from_source_label(&event.label),
            core::convert::identity,
        )
}

impl<'a> MeetCtx<'a> {
    fn type_hint(&self, event_id: i64) -> Option<&'a str> {
        self.metadata
            .and_then(|metadata| metadata.event_type(event_id))
    }
}

impl MeetCtx<'_> {
    fn walk(
        &mut self,
        results: &AllResults,
        legs: &BTreeMap<i64, Vec<&PublishedLeg>>,
        divisions: &HashMap<i64, &str>,
    ) -> crate::CrawlResult<()> {
        results.blocks.iter().fold(Ok(()), |outcome, event| {
            let projected = self.walk_event(event, legs, divisions);
            outcome.and(projected)
        })
    }

    fn walk_event(
        &mut self,
        event: &FlatEvent,
        legs: &BTreeMap<i64, Vec<&PublishedLeg>>,
        divisions: &HashMap<i64, &str>,
    ) -> crate::CrawlResult<()> {
        self.counts.blocks = self.counts.blocks.saturating_add(1);
        let Some(gender) = gender_of(&event.gender) else {
            self.counts.blocks_gender_unknown = self.counts.blocks_gender_unknown.saturating_add(1);
            return Ok(());
        };
        let kind = kind_of(event);
        self.count_metadata(&kind, event.event_id);
        let labels = [event.label.as_str(), event.short.as_str()];
        let block = Block {
            kind: &kind,
            gender,
            labels: &labels,
            type_hint: self.type_hint(event.event_id),
            division: self.division_of(event, divisions),
            round: round_of(event.round.as_deref()),
            metadata_conflict: self.metadata.and_then(|metadata| {
                (event_type_agrees(&kind, metadata, event.event_id) == Some(false)).then(|| {
                    events::MetadataConflict {
                        event_id: event.event_id,
                        declared_type: metadata.event_type(event.event_id),
                        is_hurdle: metadata.is_hurdle(event.event_id),
                    }
                })
            }),
        };
        event.results.iter().fold(Ok(()), |outcome, row| {
            self.counts.rows_seen = self.counts.rows_seen.saturating_add(1);
            let projected = if kind.is_relay() || legs.contains_key(&row.result_id) {
                self.relay_row(&block, row, legs)
            } else {
                self.individual_row(&block, row)
            };
            outcome.and(projected)
        })
    }

    fn count_metadata(&mut self, kind: &EventKind, event_id: i64) {
        if let Some(metadata) = self.metadata {
            if metadata.event_type(event_id).is_none() {
                self.counts.blocks_metadata_absent =
                    self.counts.blocks_metadata_absent.saturating_add(1);
            } else if event_type_agrees(kind, metadata, event_id) == Some(false) {
                self.counts.blocks_event_type_mismatch =
                    self.counts.blocks_event_type_mismatch.saturating_add(1);
            }
        }
    }

    fn division_of(&mut self, event: &FlatEvent, divisions: &HashMap<i64, &str>) -> Option<String> {
        let division = event
            .division
            .as_deref()
            .map(str::trim)
            .filter(|division| !division.is_empty())
            .map(str::to_string)
            .or_else(|| {
                let id = event.division_id?;
                let name = divisions.get(&id)?;
                Some(name.trim().to_string()).filter(|name| !name.is_empty())
            });
        if division.is_none() {
            self.counts.blocks_without_division =
                self.counts.blocks_without_division.saturating_add(1);
        }
        division
    }
}
