use std::collections::{BTreeMap, HashMap};

use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventId,
    EventKind, Evidence, Gender, SchoolId, SchoolYear, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

pub const SOURCE_ID: &str = "athleticlive_results";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResultStats {
    pub rows_read: usize,
    pub rows_mapped: usize,
    pub rows_skipped_no_name: usize,
    pub rows_skipped_no_school: usize,
    pub rows_skipped_unresolved_school: usize,
    pub rows_skipped_below_high_school: usize,
    pub rows_skipped_no_grade: usize,
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
    pub documents_read: usize,
    pub standings_read: usize,
    pub events_listed: usize,
    pub events_relay: usize,
    pub events_unmapped: usize,
    pub events_unfetched: usize,
    pub unresolved: BTreeMap<String, usize>,
}

impl ResultStats {
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

    fn rows_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}rows read: {} (mapped {}, skipped {})",
            self.rows_read,
            self.rows_mapped,
            self.skipped()
        )
    }

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

    fn performances_line(&self, prefix: &str) -> String {
        format!(
            "{prefix}performances: {} written, {} rows published no mark",
            self.rows_mapped.saturating_sub(self.rows_without_mark),
            self.rows_without_mark
        )
    }

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

    fn skipped(&self) -> usize {
        self.rows_skipped_no_name
            .saturating_add(self.rows_skipped_no_school)
            .saturating_add(self.rows_skipped_unresolved_school)
            .saturating_add(self.rows_skipped_below_high_school)
            .saturating_add(self.rows_skipped_no_grade)
    }
}

#[derive(Debug, Default)]
pub struct DocumentEntities {
    pub meets: Vec<CanonicalMeet>,
    pub events: Vec<CanonicalEvent>,
    pub teams: Vec<CanonicalTeam>,
    pub athletes: Vec<CanonicalAthlete>,
    pub performances: Vec<CanonicalPerformance>,
    pub stats: ResultStats,
}

#[derive(Debug, Default)]
pub(super) struct Accumulator {
    pub meets: BTreeMap<String, CanonicalMeet>,
    pub events: BTreeMap<String, CanonicalEvent>,
    pub teams: BTreeMap<String, CanonicalTeam>,
    pub athletes: BTreeMap<String, CanonicalAthlete>,
    pub performances: BTreeMap<String, CanonicalPerformance>,
}

impl Accumulator {
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

pub(super) struct Writer<'a> {
    pub index: &'a SchoolIndex,
    pub resolved: &'a mut HashMap<String, Option<SchoolId>>,
    pub stats: &'a mut ResultStats,
    pub accumulator: &'a mut Accumulator,
}

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
    pub event_key: String,
    pub provider: &'a str,
    pub jurisdiction: UsJurisdiction,
}
