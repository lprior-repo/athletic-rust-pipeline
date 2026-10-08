use std::collections::{BTreeMap, HashMap};

use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventId,
    EventKind, Evidence, Gender, ReviewCase, SchoolId, SchoolYear, SourceObservation, SourceRef,
    Sport,
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
    pub rows_skipped_unsupported_cohort: usize,
    pub rows_without_mark: usize,
    pub rows_out_of_scope_future: usize,
    pub rows_date_unknown: usize,
    pub rows_conflicting_status: usize,
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
        if self.rows_conflicting_status > 0 {
            lines.push(format!(
                "{prefix}source status/numeric conflicts: {} retained; numeric winners withheld",
                self.rows_conflicting_status
            ));
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
            "{prefix}skipped: no name {}, no school label {}, unresolved school {}, below high school {}, no grade {}, unsupported cohort {}",
            self.rows_skipped_no_name,
            self.rows_skipped_no_school,
            self.rows_skipped_unresolved_school,
            self.rows_skipped_below_high_school,
            self.rows_skipped_no_grade,
            self.rows_skipped_unsupported_cohort,
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
            .saturating_add(self.rows_skipped_unsupported_cohort)
    }
}

#[derive(Debug, Default)]
pub struct DocumentEntities {
    pub meets: Vec<CanonicalMeet>,
    pub events: Vec<CanonicalEvent>,
    pub teams: Vec<CanonicalTeam>,
    pub athletes: Vec<CanonicalAthlete>,
    pub performances: Vec<CanonicalPerformance>,
    pub review_cases: Vec<ReviewCase>,
    pub source_observations: Vec<SourceObservation>,
    pub stats: ResultStats,
}

#[derive(Debug, Default)]
pub(super) struct Accumulator {
    pub meets: BTreeMap<String, CanonicalMeet>,
    pub events: HashMap<String, CanonicalEvent>,
    pub teams: BTreeMap<String, CanonicalTeam>,
    pub athletes: BTreeMap<String, CanonicalAthlete>,
    pub performances: HashMap<String, CanonicalPerformance>,
    pub unsupported: crate::cohort::UnsupportedCohortRows,
}

impl Accumulator {
    pub(super) fn into_entities(self, stats: ResultStats) -> crate::CrawlResult<DocumentEntities> {
        let (review_cases, source_observations) = self.unsupported.into_parts();
        Ok(DocumentEntities {
            meets: self.meets.into_values().collect(),
            events: ordered(self.events)?,
            teams: self.teams.into_values().collect(),
            athletes: self.athletes.into_values().collect(),
            performances: ordered(self.performances)?,
            review_cases,
            source_observations,
            stats,
        })
    }
}

fn ordered<T>(entries: HashMap<String, T>) -> crate::CrawlResult<Vec<T>> {
    let mut keyed = Vec::new();
    keyed
        .try_reserve_exact(entries.len())
        .map_err(|_| collection_resource(entries.len()))?;
    keyed.extend(entries);
    keyed.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let mut values = Vec::new();
    values
        .try_reserve_exact(keyed.len())
        .map_err(|_| collection_resource(keyed.len()))?;
    values.extend(keyed.into_iter().map(|(_, value)| value));
    Ok(values)
}

fn collection_resource(requested: usize) -> crate::CrawlError {
    crate::CrawlError::Resource {
        resource: "LIVE ordered entities",
        requested,
        limit: 100_000,
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
    pub performance_as_of: chrono::NaiveDate,
    pub event_key: String,
    pub provider: &'a str,
    pub jurisdiction: UsJurisdiction,
}
