//! Row-level comparison against the store's canonical records.

use std::collections::{HashMap, HashSet};

use census_domain::model::{CanonicalAthlete, CanonicalPerformance, CanonicalSchool};

use census_store::{Store, Table};

/// A single discrepancy found during verification.
#[derive(Debug)]
pub struct Discrepancy {
    pub row: usize,
    pub message: String,
}

/// A verification pass over one entity type.
#[derive(Debug)]
pub struct EntityCheck {
    /// Total data rows read from the sheet(s).
    pub total_rows: usize,
    /// Number of rows that passed verification.
    pub passed: usize,
    /// Indices that were sampled.
    pub sampled_indices: Vec<usize>,
}

/// Extract a trimmed string field from a row via the column map.
fn field<'a>(col_map: &'a HashMap<&str, usize>, row: &'a [String], name: &str) -> &'a str {
    col_map
        .get(name)
        .and_then(|&i| row.get(i))
        .map(|s| s.trim())
        .unwrap_or("")
}

/// Verify athletes: every sampled row's athlete must exist in the store's Athletes table
/// with the same id, name, school id, and grad_year of 2027.
pub fn verify_athletes(
    store: &Store,
    rows: &[Vec<String>],
    sampled: &[usize],
    col_map: &HashMap<&str, usize>,
) -> Result<EntityCheck, Discrepancy> {
    let athletes: Vec<CanonicalAthlete> =
        store.scan(Table::Athletes).map_err(|source| Discrepancy {
            row: 0,
            message: format!("reading athletes from store: {source}"),
        })?;

    let school_names: HashMap<String, String> = store
        .scan(Table::Schools)
        .map_err(|source| Discrepancy {
            row: 0,
            message: format!("reading schools from store: {source}"),
        })?
        .into_iter()
        .map(|school: CanonicalSchool| (school.id.as_str().to_string(), school.name))
        .collect();

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            row: idx,
            message: "row index out of range".to_string(),
        })?;
        check_athlete_row(idx, row, &athletes, &school_names, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        total_rows: rows.len(),
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

/// Check one sampled athlete row against the store: the id must exist, the name must match, the
/// School cell must resolve the way the sheet prints it, and the row must be the published cohort.
fn check_athlete_row(
    idx: usize,
    row: &[String],
    athletes: &[CanonicalAthlete],
    school_names: &HashMap<String, String>,
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let aid = field(col_map, row, "Athlete ID");
    let name = field(col_map, row, "Name");
    let store_athlete = athletes
        .iter()
        .find(|a| a.id.as_str() == aid)
        .ok_or_else(|| Discrepancy {
            row: idx,
            message: format!("athletes row {idx}: id {aid} not in store"),
        })?;

    if store_athlete.canonical_name != name {
        return Err(Discrepancy {
            row: idx,
            message: format!(
                "athletes row {idx}: id {aid} name '{name}' != store '{}'",
                store_athlete.canonical_name,
            ),
        });
    }
    check_athlete_school(
        idx,
        aid,
        field(col_map, row, "School"),
        store_athlete,
        school_names,
    )?;
    check_athlete_cohort(idx, aid, row, col_map)
}

/// The Athletes sheet prints the athlete's school by *name* — that is the recruiter's view — so the
/// check resolves that name through the row the athlete names rather than comparing a printed name
/// against a set of ids, which would fail every row. A school renamed in the store after the
/// workbook was written is still a discrepancy, which is the point.
///
/// The store holds no row for some school ids, so there is no name to print and the sheet carries
/// the id itself — the recruiting read model's documented fallback (`Dataset::school_name`). The
/// check holds the workbook to that and to nothing else: an id printed where the store *does* hold
/// a row is still a discrepancy, which is what catches an athlete published with its school
/// filtered out of the sheet's own index.
fn check_athlete_school(
    idx: usize,
    aid: &str,
    school: &str,
    store_athlete: &CanonicalAthlete,
    school_names: &HashMap<String, String>,
) -> Result<(), Discrepancy> {
    let school_id = store_athlete.school.as_str();
    match school_names.get(school_id) {
        Some(store_school) if store_school == school => Ok(()),
        Some(store_school) => Err(Discrepancy {
            row: idx,
            message: format!(
                "athletes row {idx}: id {aid} school '{school}' != store '{store_school}'"
            ),
        }),
        None if school == school_id => Ok(()),
        None => Err(Discrepancy {
            row: idx,
            message: format!(
                "athletes row {idx}: id {aid} school row '{school_id}' not in store, and the sheet \
                 prints '{school}'"
            ),
        }),
    }
}

/// The Graduation Year cell must name the cohort the run published.
fn check_athlete_cohort(
    idx: usize,
    aid: &str,
    row: &[String],
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let grad_year = col_map
        .get("Graduation Year")
        .and_then(|&i| row.get(i))
        .and_then(|s| s.trim().parse::<i16>().ok());
    if grad_year == Some(2027) {
        return Ok(());
    }
    Err(Discrepancy {
        row: idx,
        message: format!(
            "athletes row {idx}: id {aid} grad_year {} != 2027",
            grad_year.map_or("?".to_string(), |y| y.to_string()),
        ),
    })
}

/// Verify performances: every sampled row's (athlete id, event, mark) must exist in the store.
pub fn verify_performances(
    store: &Store,
    rows: &[Vec<String>],
    sampled: &[usize],
    col_map: &HashMap<&str, usize>,
) -> Result<EntityCheck, Discrepancy> {
    let performances: Vec<CanonicalPerformance> =
        store
            .scan(Table::Performances)
            .map_err(|source| Discrepancy {
                row: 0,
                message: format!("reading performances from store: {source}"),
            })?;

    // Build a lookup of (athlete_id, event_id, mark) for fast matching.
    let mut lookup: HashSet<(String, String, String)> = HashSet::new();
    for perf in &performances {
        let mark_str = match &perf.mark {
            census_domain::model::Mark::TimeSeconds(t) => format!("{t}"),
            census_domain::model::Mark::DistanceMetres(d) => format!("{d}"),
            census_domain::model::Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
            census_domain::model::Mark::Points(p) => format!("{p}"),
            census_domain::model::Mark::Raw(r) => r.clone(),
        };
        lookup.insert((
            perf.athlete.as_str().to_string(),
            perf.event.as_str().to_string(),
            mark_str,
        ));
    }

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            row: idx,
            message: "row index out of range".to_string(),
        })?;
        check_performance_row(idx, row, &lookup, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        total_rows: rows.len(),
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

/// Check one sampled performance row against the store lookup.
fn check_performance_row(
    idx: usize,
    row: &[String],
    lookup: &HashSet<(String, String, String)>,
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let aid = field(col_map, row, "Athlete ID");
    let event = field(col_map, row, "Event");
    let mark = field(col_map, row, "Mark");

    let key = (aid.to_string(), event.to_string(), mark.to_string());
    if !lookup.contains(&key) {
        return Err(Discrepancy {
            row: idx,
            message: format!(
                "performances row {idx}: id {aid} event '{event}' mark '{mark}' not in store"
            ),
        });
    }
    Ok(())
}
