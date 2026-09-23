//! Canonical mapping for the result plane: one document's rows become events and performances.
//!
//! Identity rules this module holds to (the row-level fold is `map_rows::record_row`, one entry
//! point per route, which decodes a row into the raw facts below and then mints):
//!
//! * Athletes come from `(school, name, grad year, gender)` and teams from
//!   `(school, sport, gender, school year)` — the same keys the roster, association and result-file
//!   adapters mint, so an AthleticLIVE result reconciles with an athlete the census already holds
//!   instead of creating a parallel one.
//! * The meet is minted from the harvest's own target (`state` + `date` + `name`), never from the
//!   result payload, so it lands on the id `meets::build_meets` already wrote for the same meet.
//! * Schools are never minted here. A published team label is resolved against the consolidated
//!   school index; a label that names no consolidated school (a club team, `Unattached`) is counted
//!   and skipped rather than invented into the school table.
//! * A row that publishes no high-school grade is counted and skipped: the census is the
//!   Class-of-2027 cohort, and a below-high-school grade (an open meet allows one) is not a cohort
//!   member. Every skip is counted by reason and published in the run's notes.
//! * `a.ani` / `t.ani` (event documents) and `ani` (standings) become `LegacyAthleticNet` identity
//!   rows with the profile URL the id addresses; `anli` is read and counted but never minted,
//!   because its athlete-level semantics are `[I]` in `[sources/state-assoc-plains]`, not measured.
//! * Evidence: every entity minted here carries
//!   `SourceRef::new("athleticlive_results", Some(<document url>))` — the id the report's non-core
//!   list has to name alongside `athleticlive_meets_csv` and `athleticlive_athletes`.

use std::collections::{BTreeMap, HashMap};

use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventId,
    EventKind, Evidence, Gender, SchoolId, SchoolYear, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

/// The source id every entity and evidence row this adapter writes carries.
pub const SOURCE_ID: &str = "athleticlive_results";

/// The run counters the notes publish.
///
/// Every field saturates on increment: a wrap would silently turn a large run into a small number
/// in the report, and the report is what a coverage claim rests on.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResultStats {
    pub rows_read: usize,
    pub rows_mapped: usize,
    pub rows_skipped_no_name: usize,
    pub rows_skipped_no_school: usize,
    pub rows_skipped_unresolved_school: usize,
    pub rows_skipped_below_high_school: usize,
    pub rows_skipped_no_grade: usize,
    /// Mapped rows that published no mark: identity evidence without a performance.
    pub rows_without_mark: usize,
    pub rows_with_athlete_id: usize,
    pub rows_with_timer_team_id: usize,
    pub rows_with_an_team_id: usize,
    pub rows_with_legacy_id: usize,
    pub rows_with_timer_team_key: usize,
    pub rows_with_splits: usize,
    pub splits: usize,
    pub rows_with_seed: usize,
    pub rows_with_wind: usize,
    pub rows_with_heat: usize,
    pub rows_unplaced: usize,
    /// Event documents absorbed, one request each.
    pub documents_read: usize,
    /// Standings payloads absorbed, one request each.
    pub standings_read: usize,
    /// Summary events listed, and what the listing refused.
    pub events_listed: usize,
    pub events_relay: usize,
    pub events_unmapped: usize,
    pub events_unfetched: usize,
    /// School labels that resolved to no consolidated school, by label.
    pub unresolved: BTreeMap<String, usize>,
}

impl ResultStats {
    /// Lines for the run's notes: the row ledger, then the identity channels, then what was
    /// skipped. Counts are printed unconditionally so a zero stays visible as a measurement.
    pub fn note(&self, prefix: &str) -> Vec<String> {
        let mut lines = vec![
            self.rows_line(prefix),
            self.skipped_line(prefix),
            self.performances_line(prefix),
            self.channels_line(prefix),
            self.splits_line(prefix),
            self.documents_line(prefix),
        ];
        if let Some(line) = self.unresolved_line(prefix) {
            lines.push(line);
        }
        lines
    }

    /// Rows read, and how many of them yielded entities.
    fn rows_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}rows read: {} (mapped {}, skipped {})",
            self.rows_read,
            self.rows_mapped,
            self.skipped()
        )
    }

    /// Every refusal reason, by count.
    fn skipped_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}skipped: no name {}, no school label {}, unresolved school {}, below high school {}, no grade {}",
            self.rows_skipped_no_name,
            self.rows_skipped_no_school,
            self.rows_skipped_unresolved_school,
            self.rows_skipped_below_high_school,
            self.rows_skipped_no_grade
        )
    }

    /// Performances written, and the mapped rows that published no mark for them.
    fn performances_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}performances: {} written, {} rows published no mark",
            self.rows_mapped.saturating_sub(self.rows_without_mark),
            self.rows_without_mark
        )
    }

    /// The identity channels a mapped row publishes, counted per channel.
    fn channels_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}identity channels: athletic.net athlete ids {}, athleticlive team ids {}, athletic.net team ids {}, legacy ids {}, short team keys {}",
            self.rows_with_athlete_id,
            self.rows_with_timer_team_id,
            self.rows_with_an_team_id,
            self.rows_with_legacy_id,
            self.rows_with_timer_team_key
        )
    }

    /// The channels a mapped row publishes beside its mark.
    fn splits_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}splits: {} rows carrying {} splits; seeds {}, wind {}, heats {}, unplaced {}",
            self.rows_with_splits,
            self.splits,
            self.rows_with_seed,
            self.rows_with_wind,
            self.rows_with_heat,
            self.rows_unplaced
        )
    }

    /// The captures the run read, and the events a summary listed that no document carried.
    fn documents_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}documents: {} event documents, {} standings; events listed {} (relay {}, unmapped {}, unfetched {})",
            self.documents_read,
            self.standings_read,
            self.events_listed,
            self.events_relay,
            self.events_unmapped,
            self.events_unfetched
        )
    }

    /// The school labels that resolved to nothing, most restrictive list first, capped at ten.
    fn unresolved_line(&self, prefix: &str) -> Option<String> {
        let labels: Vec<String> = self
            .unresolved
            .iter()
            .take(10)
            .map(|(label, count)| format!("{label} x{count}"))
            .collect();
        (!labels.is_empty()).then(|| {
            format!(
                "{prefix}unresolved school labels (up to 10): {}",
                labels.join(", ")
            )
        })
    }

    /// Every refusal reason, summed.
    fn skipped(&self) -> usize {
        self.rows_skipped_no_name
            .saturating_add(self.rows_skipped_no_school)
            .saturating_add(self.rows_skipped_unresolved_school)
            .saturating_add(self.rows_skipped_below_high_school)
            .saturating_add(self.rows_skipped_no_grade)
    }
}

/// Canonical entities one result document or race yields.
#[derive(Debug, Default)]
pub struct DocumentEntities {
    pub meets: Vec<CanonicalMeet>,
    pub events: Vec<CanonicalEvent>,
    pub teams: Vec<CanonicalTeam>,
    pub athletes: Vec<CanonicalAthlete>,
    pub performances: Vec<CanonicalPerformance>,
    pub stats: ResultStats,
}

/// Entities accumulated across a walk, keyed by canonical id so a re-publication merges instead of
/// duplicating (the same rule the store's tables apply).
#[derive(Debug, Default)]
pub(super) struct Accumulator {
    pub meets: BTreeMap<String, CanonicalMeet>,
    pub events: BTreeMap<String, CanonicalEvent>,
    pub teams: BTreeMap<String, CanonicalTeam>,
    pub athletes: BTreeMap<String, CanonicalAthlete>,
    pub performances: BTreeMap<String, CanonicalPerformance>,
}

impl Accumulator {
    /// Unpack into one vector per table, for a single append per table.
    pub(super) fn into_entities(self, stats: ResultStats) -> DocumentEntities {
        DocumentEntities {
            meets: self.meets.into_values().collect(),
            events: self.events.into_values().collect(),
            teams: self.teams.into_values().collect(),
            athletes: self.athletes.into_values().collect(),
            performances: self.performances.into_values().collect(),
            stats,
        }
    }
}

/// The state one walk carries across documents: the school index, the resolved-label memo (a
/// label is resolved once per run, not once per row), the counters and the entities so far.
pub(super) struct Writer<'a> {
    pub index: &'a SchoolIndex,
    pub resolved: &'a mut HashMap<String, Option<SchoolId>>,
    pub stats: &'a mut ResultStats,
    pub accumulator: &'a mut Accumulator,
}

/// The event a row belongs to, as the row's mapper needs it.
pub(super) struct RowContext<'a> {
    pub meet: &'a CanonicalMeet,
    pub source: &'a SourceRef,
    pub evidence: &'a Evidence,
    pub event_id: &'a EventId,
    pub kind: &'a EventKind,
    pub gender: Gender,
    pub round: Option<String>,
    pub sport: Sport,
    pub school_year: SchoolYear,
    /// `athleticlive:<event id>`: the prefix of every row key this event mints, so the two routes
    /// that can publish one race (`ind_res_list/_doc/<id>`, `liveRunStandings/<runId>`) agree on
    /// the performance key when they publish the same athlete.
    pub event_key: String,
    /// The timer tenant that published the meet, which is the namespace of a team's timer id.
    pub provider: &'a str,
    /// The jurisdiction the meet was held in: a row's published label is resolved inside it, so a
    /// label can only ever match a school of that state.
    pub jurisdiction: UsJurisdiction,
}
