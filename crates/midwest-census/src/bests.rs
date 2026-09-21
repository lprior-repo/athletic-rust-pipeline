//! Per-athlete best marks - the platform's own "PR" reduction.
//!
//! A best mark is a deterministic reduction over rows the platform already reconciled, so nothing here
//! fetches anything: for every `(athlete, event)` pair it keeps the mark that wins on that event's own
//! scale - the lowest time, the highest distance, height or score - and carries along the meet, date,
//! place, wind reading and timing method that produced it. A recruiting row can therefore cite the
//! performance instead of asserting a number.
//!
//! Two deliberate choices:
//!
//! * **Relays are not personal bests.** A 4x400 split is a squad mark; including it would put a
//!   number in an athlete's PR column that the athlete did not run alone.
//! * **Marks are compared only within their own measure.** A time is never compared against a
//!   distance, so an unparsed [`Mark::Raw`] value is carried but never chosen as a best.

use crate::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventKind, Mark,
};
use crate::report::{retain_core, Scope};
use crate::store::{Store, Table};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// One athlete's best mark in one event, with the performance that produced it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BestResult {
    pub athlete_id: String,
    pub name: String,
    pub school: String,
    pub state: String,
    pub grad_year: i16,
    pub gender: String,
    pub sport: String,
    pub event: String,
    pub best_mark: String,
    pub best_value: f64,
    pub measure: String,
    pub date: String,
    pub meet: String,
    pub place: Option<u16>,
    pub wind_mps: Option<f64>,
    pub timing: Option<String>,
    /// How many marks this athlete has in this event, i.e. how much the best rests on.
    pub marks_in_event: usize,
    pub profile_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub scope: Scope,
    /// Cohort selector; `None` reduces every athlete in scope.
    pub grad_year: Option<i16>,
    pub limit: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        }
    }
}

/// How a mark is compared. Nothing crosses measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measure {
    /// Lower is better.
    Time,
    /// Higher is better (metric distance or height).
    Distance,
    /// Higher is better, preserved in the source's feet-and-inches notation.
    Field,
    /// Higher is better (combined-event or team score).
    Points,
}

impl Measure {
    pub fn of(mark: &Mark) -> Option<Self> {
        match mark {
            Mark::TimeSeconds(_) => Some(Measure::Time),
            Mark::DistanceMetres(_) => Some(Measure::Distance),
            Mark::FieldImperial { .. } => Some(Measure::Field),
            Mark::Points(_) => Some(Measure::Points),
            Mark::Raw(_) => None,
        }
    }

    /// The comparable number, in this measure's own unit.
    pub fn value(self, mark: &Mark) -> Option<f64> {
        match (self, mark) {
            (Measure::Time, Mark::TimeSeconds(seconds)) => Some(*seconds),
            (Measure::Distance, Mark::DistanceMetres(metres)) => Some(*metres),
            (Measure::Field, Mark::FieldImperial { metres, .. }) => Some(*metres),
            (Measure::Points, Mark::Points(points)) => Some(*points),
            _ => None,
        }
    }

    pub const fn better(self, candidate: f64, incumbent: f64) -> bool {
        match self {
            Measure::Time => candidate < incumbent,
            Measure::Distance | Measure::Field | Measure::Points => candidate > incumbent,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Measure::Time => "time",
            Measure::Distance => "distance",
            Measure::Field => "field",
            Measure::Points => "points",
        }
    }
}

/// `true` for squad events, which are never an athlete's personal best.
pub const fn is_relay(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Relay4x100
            | EventKind::Relay4x200
            | EventKind::Relay4x400
            | EventKind::Relay4x800
            | EventKind::SprintMedley
            | EventKind::DistanceMedley
    )
}

/// Which of the platform's sports an event belongs to.
pub const fn sport_of(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::CrossCountry => "CrossCountry",
        EventKind::HighJump
        | EventKind::LongJump
        | EventKind::TripleJump
        | EventKind::PoleVault
        | EventKind::ShotPut
        | EventKind::Discus
        | EventKind::Javelin
        | EventKind::Hammer
        | EventKind::WeightThrow
        | EventKind::Pentathlon
        | EventKind::Heptathlon
        | EventKind::Decathlon => "Field",
        EventKind::Unmapped { .. } => "Unmapped",
        _ => "Track",
    }
}

/// Published notation for a mark: `10.94`, `4:41.23`, `5' 4"`, `42.10 m`, `3120 pts`.
pub fn mark_text(mark: &Mark) -> String {
    match mark {
        Mark::TimeSeconds(seconds) => format_time(*seconds),
        Mark::DistanceMetres(metres) => format!("{metres:.2} m"),
        Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
        Mark::Points(points) => format!("{points:.0} pts"),
        Mark::Raw(text) => text.clone(),
    }
}

/// Seconds as a race time: `10.94` stays `10.94`, `281.23` becomes `4:41.23`.
pub fn format_time(seconds: f64) -> String {
    if seconds < 60.0 {
        return format!("{seconds:.2}");
    }
    let minutes = (seconds / 60.0).floor();
    let remainder = seconds - minutes * 60.0;
    if remainder < 10.0 {
        format!("{minutes:.0}:0{remainder:.2}")
    } else {
        format!("{minutes:.0}:{remainder:.2}")
    }
}

/// Reduce the consolidated tables to one best mark per `(athlete, event)`.
pub fn build(store: &Store, options: &Options) -> Result<Vec<BestResult>> {
    // Read the store itself: the report reads the store too, and a snapshot left behind by an older
    // `consolidate` would silently disagree with it.
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
    let mut performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    if options.scope == Scope::Core {
        retain_core(&mut athletes);
        retain_core(&mut meets);
        retain_core(&mut events);
        retain_core(&mut performances);
    }

    let cohort: HashMap<&str, &CanonicalAthlete> =
        athletes.iter().map(|a| (a.id.as_str(), a)).collect();
    let meet_of: HashMap<&str, &CanonicalMeet> = meets.iter().map(|m| (m.id.as_str(), m)).collect();
    let kind_of: HashMap<&str, &EventKind> =
        events.iter().map(|e| (e.id.as_str(), &e.kind)).collect();

    let mut bests: HashMap<(String, String), BestResult> = HashMap::new();
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for performance in &performances {
        let Some(athlete) = cohort.get(performance.athlete.as_str()) else {
            continue;
        };
        if let Some(year) = options.grad_year {
            if athlete.grad_year.get() != year {
                continue;
            }
        }
        let Some(kind) = kind_of.get(performance.event.as_str()) else {
            continue;
        };
        if is_relay(kind) {
            continue;
        }
        let Some(measure) = Measure::of(&performance.mark) else {
            continue;
        };
        let Some(value) = measure.value(&performance.mark) else {
            continue;
        };
        let event_label = format!("{kind:?}");
        let key = (athlete.id.as_str().to_string(), event_label.clone());
        // A count is bounded by the scanned performance rows, so saturation is unreachable; it is
        // here so a change to that bound can never wrap the counter.
        let counter = counts.entry(key.clone()).or_insert(0);
        *counter = counter.saturating_add(1);

        let wins = bests
            .get(&key)
            .map(|incumbent| measure.better(value, incumbent.best_value))
            .unwrap_or(true);
        if !wins {
            continue;
        }
        let meet = meet_of.get(performance.meet.as_str());
        bests.insert(
            key,
            BestResult {
                athlete_id: athlete.id.as_str().to_string(),
                name: athlete.canonical_name.clone(),
                school: athlete.school.as_str().to_string(),
                state: meet
                    .map(|meet| meet.state.clone())
                    .unwrap_or_else(|| "??".to_string()),
                grad_year: athlete.grad_year.get(),
                gender: format!("{:?}", athlete.gender),
                sport: sport_of(kind).to_string(),
                event: event_label,
                best_mark: mark_text(&performance.mark),
                best_value: value,
                measure: measure.as_str().to_string(),
                date: performance.date.clone(),
                meet: meet.map(|meet| meet.name.clone()).unwrap_or_default(),
                place: performance.place,
                wind_mps: performance.wind_mps,
                timing: performance.timing.map(|timing| format!("{timing:?}")),
                marks_in_event: 0,
                profile_url: athlete.public_profile_urls.first().cloned(),
            },
        );
    }

    let mut rows: Vec<BestResult> = bests
        .into_iter()
        .map(|(key, mut row)| {
            row.marks_in_event = counts.get(&key).copied().unwrap_or(1);
            row
        })
        .collect();
    // Best first inside each event; times ascend, everything else descends.
    rows.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| {
                if left.measure == "time" {
                    left.best_value.total_cmp(&right.best_value)
                } else {
                    right.best_value.total_cmp(&left.best_value)
                }
            })
            .then_with(|| left.name.cmp(&right.name))
    });
    if let Some(limit) = options.limit {
        rows.truncate(limit);
    }
    Ok(rows)
}

/// Write the reduction as `out/best-results-<cohort>.jsonl` and `.csv`.
pub fn write(store: &Store, rows: &[BestResult], cohort: &str) -> Result<(PathBuf, PathBuf)> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let jsonl = out.join(format!("best-results-{cohort}.jsonl"));
    let csv_path = out.join(format!("best-results-{cohort}.csv"));

    let mut json = std::io::BufWriter::new(std::fs::File::create(&jsonl)?);
    for row in rows {
        serde_json::to_writer(&mut json, row)?;
        std::io::Write::write_all(&mut json, b"\n")?;
    }
    std::io::Write::flush(&mut json)?;

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&csv_path)?;
    writer.write_record([
        "athlete_id",
        "name",
        "school",
        "state",
        "grad_year",
        "gender",
        "sport",
        "event",
        "best_mark",
        "best_value",
        "measure",
        "date",
        "meet",
        "place",
        "wind_mps",
        "timing",
        "marks_in_event",
        "profile_url",
    ])?;
    for row in rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    Ok((jsonl, csv_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sidecar CSV declares its own header. `csv::Writer` defaults to `has_headers(true)`, which
    /// makes the first `serialize` emit a second, derived header, so every consumer saw the header
    /// twice. This fails if that default is restored.
    #[test]
    fn the_csv_carries_exactly_one_header_row() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let rows = vec![BestResult {
            athlete_id: "ath_0000000000000001".to_string(),
            name: "Ada Fixture".to_string(),
            school: "sch_0000000000000001".to_string(),
            state: "AK".to_string(),
            grad_year: 2027,
            gender: "Girls".to_string(),
            sport: "CrossCountry".to_string(),
            event: "CrossCountry".to_string(),
            best_mark: "15:40.12".to_string(),
            best_value: 940.12,
            measure: "Time".to_string(),
            date: "2023-09-09".to_string(),
            meet: "Fixture Invitational".to_string(),
            place: Some(1),
            wind_mps: None,
            timing: Some("FAT".to_string()),
            marks_in_event: 1,
            profile_url: None,
        }];
        let (_, csv) = write(&store, &rows, "co2027").unwrap();
        let text = std::fs::read_to_string(&csv).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "one header plus one row, saw:\n{text}");
        assert!(lines[0].starts_with("athlete_id,name"), "{}", lines[0]);
        assert!(
            lines[1].starts_with("ath_0000000000000001,"),
            "the row follows the header directly, saw: {}",
            lines[1]
        );
    }

    #[test]
    fn times_run_down_and_field_marks_run_up() {
        assert!(Measure::Time.better(10.94, 11.02));
        assert!(!Measure::Time.better(11.02, 10.94));
        assert!(Measure::Distance.better(6.42, 6.10));
        assert!(!Measure::Distance.better(6.10, 6.42));
        assert!(Measure::Field.better(1.85, 1.70));
        assert!(Measure::Points.better(3120.0, 2900.0));
        // Unparsed marks are carried but never chosen.
        assert_eq!(Measure::of(&Mark::Raw("DNS".to_string())), None);
    }

    #[test]
    fn marks_print_in_the_notation_a_reader_expects() {
        assert_eq!(mark_text(&Mark::TimeSeconds(10.94)), "10.94");
        assert_eq!(mark_text(&Mark::TimeSeconds(281.23)), "4:41.23");
        assert_eq!(mark_text(&Mark::TimeSeconds(304.1)), "5:04.10");
        assert_eq!(mark_text(&Mark::DistanceMetres(6.4213)), "6.42 m");
        assert_eq!(
            mark_text(&Mark::FieldImperial {
                feet_mark: "5' 4\"".to_string(),
                metres: 1.63,
            }),
            "5' 4\""
        );
        assert_eq!(mark_text(&Mark::Points(3120.0)), "3120 pts");
    }

    #[test]
    fn relays_are_not_personal_bests() {
        assert!(is_relay(&EventKind::Relay4x400));
        assert!(is_relay(&EventKind::SprintMedley));
        assert!(!is_relay(&EventKind::Track400m));
        assert!(!is_relay(&EventKind::CrossCountry));
    }
}
