//! One meet's walk: every block, and the block-level context its rows are read against. The rows
//! themselves live in [`rows`].
//!
//! The walk refuses what the payload does not publish and counts every refusal:
//!
//! * a meet the payload does not place in one of the 51 jurisdictions (`Overseas` publishes no
//!   state code) or does not date is dropped whole;
//! * a block with no published gender, or whose label disagrees with the type the metadata
//!   document declares, is counted and never walked;
//! * a row is refused, in this order, when it has no school the payload names, no athlete id, no
//!   grade in `9`..=`12`, no name, no readable mark, or belongs to an event whose label maps to no
//!   platform kind while the metadata document was not spent — the last case is the only one the
//!   third request changes, and it is why the request exists at all.
//!
//! Relay squads are never attributed as individual rows: the squad row's own row is skipped (its
//! `FirstName` is `<BR>`-joined markup, its `LastName` null) and every leg the payload publishes
//! for it becomes one performance carrying the squad's mark and place.

mod rows;

use super::count::MeetStats;
use super::read::{
    event_type_agrees, jurisdiction_of, legs_by_result, meet_date, sport_of, EventMetadata,
};
use super::store::meet_row;
use super::wire::{AllResults, FlatEvent, MeetData, PublishedLeg};
use crate::school_index::SchoolIndex;
use crate::sources::athleticnet::map::{Accumulator, Stats};
use crate::sources::athleticnet::parse::{gender_of, round_of};
use census_domain::model::{CanonicalMeet, EventKind, SchoolId, SchoolYear, SourceRef};
use census_domain::UsJurisdiction;
use rows::Block;
use std::collections::{BTreeMap, HashMap};

/// Absorb one whole meet; returns the performance rows stored and what the walk counted.
#[allow(clippy::too_many_arguments)]
pub(in crate::sources::athleticnet) fn absorb_meet(
    meet: &MeetData,
    results: &AllResults,
    metadata: Option<&EventMetadata>,
    source: &SourceRef,
    observed_on: &str,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, SchoolId>,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> (u64, MeetStats) {
    let mut counts = MeetStats::default();
    let Some(state) = meet_state(meet) else {
        counts.meets_unplaced = counts.meets_unplaced.saturating_add(1);
        return (0, counts);
    };
    let Some(date) = meet_date(&meet.meet.date) else {
        counts.meets_without_date = counts.meets_without_date.saturating_add(1);
        return (0, counts);
    };
    let date = date.to_string();
    let Some(season) = meet.meet.season_id.or_else(|| season_of(&date)) else {
        counts.meets_without_season = counts.meets_without_season.saturating_add(1);
        return (0, counts);
    };
    // A published season the domain will not place is refused exactly as an absent one: the meet's
    // rows are not filed under a year no source published.
    let Some(school_year) = SchoolYear::containing(season, 5) else {
        counts.meets_without_season = counts.meets_without_season.saturating_add(1);
        return (0, counts);
    };
    let sport = sport_of(meet.sport2.as_deref());
    let divisions: HashMap<i64, &str> = meet
        .divisions
        .iter()
        .map(|division| (division.id, division.name.as_str()))
        .collect();
    let legs = legs_by_result(&results.legs);
    let row = meet_row(meet, state, &date, sport, source, observed_on, accumulated);
    let mut ctx = MeetCtx {
        source,
        observed_on,
        index,
        resolved,
        stats,
        accumulated,
        counts: &mut counts,
        metadata,
        state,
        sport,
        school_year,
        date,
        meet: row,
        school_names: results
            .teams
            .iter()
            .map(|team| (team.school_id.to_string(), team.school_name.as_str()))
            .collect(),
    };
    ctx.walk(results, &legs, &divisions);
    counts.meets_pulled = counts.meets_pulled.saturating_add(1);
    let stored = counts.stored();
    (stored, counts)
}

/// The jurisdiction the meet document places the venue in, if any.
fn meet_state(meet: &MeetData) -> Option<UsJurisdiction> {
    jurisdiction_of(
        meet.meet
            .location
            .as_ref()
            .and_then(|location| location.state.as_deref()),
    )
}

/// The season year a published meet date carries, used when the payload publishes no `SeasonID`.
fn season_of(date: &str) -> Option<i16> {
    date.split('-').next()?.trim().parse::<i16>().ok()
}

/// One meet's walk: the meet it resolved, the names its rows resolve schools against, and where its
/// rows go.
struct MeetCtx<'a> {
    source: &'a SourceRef,
    observed_on: &'a str,
    index: &'a SchoolIndex,
    resolved: &'a mut HashMap<String, SchoolId>,
    stats: &'a mut Stats,
    accumulated: &'a mut Accumulator,
    counts: &'a mut MeetStats,
    metadata: Option<&'a EventMetadata>,
    state: UsJurisdiction,
    sport: census_domain::model::Sport,
    school_year: SchoolYear,
    date: String,
    meet: CanonicalMeet,
    /// School names by the id a row carries as `TeamID`, published once per meet.
    school_names: HashMap<String, &'a str>,
}

/// The kind a block's label maps to: the short code the ontology is keyed on, or the display label
/// when the payload publishes no short code.
fn kind_of(event: &FlatEvent) -> EventKind {
    let short = event.short.trim();
    if short.is_empty() {
        EventKind::from_source_label(&event.label)
    } else {
        EventKind::from_source_label(short)
    }
}

impl<'a> MeetCtx<'a> {
    /// The event type the metadata document declares for an event id.
    ///
    /// Tied to the document's own lifetime, not to the borrow of this context: the block readers
    /// hold the hint while they take `&mut self`.
    fn type_hint(&self, event_id: i64) -> Option<&'a str> {
        self.metadata
            .and_then(|metadata| metadata.event_type(event_id))
    }
}

impl MeetCtx<'_> {
    /// Walk every block in published order, each row into its own performance.
    fn walk(
        &mut self,
        results: &AllResults,
        legs: &BTreeMap<i64, Vec<&PublishedLeg>>,
        divisions: &HashMap<i64, &str>,
    ) {
        let metadata = self.metadata;
        for event in &results.blocks {
            self.counts.blocks = self.counts.blocks.saturating_add(1);
            let Some(gender) = gender_of(&event.gender) else {
                self.counts.blocks_gender_unknown =
                    self.counts.blocks_gender_unknown.saturating_add(1);
                continue;
            };
            let kind = kind_of(event);
            if let Some(metadata) = metadata {
                if metadata.event_type(event.event_id).is_none() {
                    self.counts.blocks_metadata_absent =
                        self.counts.blocks_metadata_absent.saturating_add(1);
                } else if event_type_agrees(&kind, metadata, event.event_id) == Some(false) {
                    self.counts.blocks_event_type_mismatch =
                        self.counts.blocks_event_type_mismatch.saturating_add(1);
                }
            }
            let block = Block {
                kind: &kind,
                gender,
                label: event.source_label(),
                type_hint: self.type_hint(event.event_id),
                division: self.division_of(event, divisions),
                round: round_of(event.round.as_deref()),
            };
            for row in &event.results {
                self.counts.rows_seen = self.counts.rows_seen.saturating_add(1);
                if kind.is_relay() || legs.contains_key(&row.result_id) {
                    self.relay_row(&block, row, legs);
                } else {
                    self.individual_row(&block, row);
                }
            }
        }
    }

    /// Whether the block's own label maps to no platform kind while the metadata document was not
    /// spent: its marks are then unreadable, and the row is refused rather than guessed at.
    fn refuse_unmapped_label(&mut self, block: &Block<'_>) -> bool {
        if matches!(block.kind, EventKind::Unmapped { .. }) && block.type_hint.is_none() {
            self.counts.rows_unmapped_event = self.counts.rows_unmapped_event.saturating_add(1);
            return true;
        }
        false
    }

    /// The division a block publishes, falling back to the payload's division table.
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
