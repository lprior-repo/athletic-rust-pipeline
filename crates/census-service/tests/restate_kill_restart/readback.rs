use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam,
};
use census_store::{Store, Table};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    Succeeded,
    Pending(String),
    Refused(String),
}

pub(super) fn snapshot_file(dir: &Path, table: Table) -> PathBuf {
    dir.join(format!("{}.jsonl", table.file()))
}

fn read_rows(path: &Path) -> Result<Vec<String>, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("{} is absent or unreadable: {error}", path.display()))?;
    if body.is_empty() {
        return Ok(Vec::new());
    }
    let complete = body
        .strip_suffix('\n')
        .ok_or_else(|| format!("{} does not end in a complete row", path.display()))?;
    let rows: Vec<String> = complete.split('\n').map(str::to_string).collect();
    for (index, row) in rows.iter().enumerate() {
        if row.trim().is_empty() {
            return Err(format!("{} row {} is blank", path.display(), index + 1));
        }
    }
    Ok(rows)
}

pub(super) fn snapshot_rows<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, String> {
    read_rows(path)?
        .iter()
        .enumerate()
        .map(|(index, row)| {
            serde_json::from_str(row).map_err(|error| {
                format!(
                    "{} row {} is malformed: {error}; row={row}",
                    path.display(),
                    index + 1
                )
            })
        })
        .collect()
}

pub(super) fn snapshot_line_count(path: &Path) -> Result<usize, String> {
    let rows = read_rows(path)?;
    for (index, row) in rows.iter().enumerate() {
        serde_json::from_str::<Value>(row).map_err(|error| {
            format!(
                "{} row {} is malformed: {error}; row={row}",
                path.display(),
                index + 1
            )
        })?;
    }
    Ok(rows.len())
}

fn canonical<T: Serialize + DeserializeOwned>(row: &T) -> Result<String, String> {
    let encoded =
        serde_json::to_value(row).map_err(|error| format!("a row is not serializable: {error}"))?;
    let typed: T = serde_json::from_value(encoded)
        .map_err(|error| format!("a row does not survive its own encoding: {error}"))?;
    let value = serde_json::to_value(&typed)
        .map_err(|error| format!("a row is not serializable: {error}"))?;
    serde_json::to_string(&value).map_err(|error| format!("a row is not encodable: {error}"))
}

fn multiset<T: Serialize + DeserializeOwned>(
    rows: &[T],
) -> Result<BTreeMap<String, usize>, String> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(canonical(row)?).or_insert(0_usize) += 1;
    }
    Ok(counts)
}

pub(super) fn compare_rows<T: Serialize + DeserializeOwned>(
    label: &str,
    expected: &[T],
    actual: &[T],
) -> Result<(), String> {
    let expected = multiset(expected)?;
    let actual = multiset(actual)?;
    if expected == actual {
        return Ok(());
    }
    let mut missing = Vec::new();
    for (row, count) in &expected {
        let found = actual.get(row).copied().unwrap_or(0);
        if found < *count {
            missing.push(format!("{row} x{}", *count - found));
        }
    }
    let mut extra = Vec::new();
    for (row, count) in &actual {
        let wanted = expected.get(row).copied().unwrap_or(0);
        if *count > wanted {
            extra.push(format!("{row} x{}", *count - wanted));
        }
    }
    missing.truncate(3);
    extra.truncate(3);
    Err(format!(
        "{label} rows differ: missing=[{}] extra=[{}]",
        missing.join(", "),
        extra.join(", ")
    ))
}

fn compare_table<T: Serialize + DeserializeOwned>(
    dir: &Path,
    table: Table,
    expected: &[T],
) -> Result<(), String> {
    let actual = snapshot_rows(&snapshot_file(dir, table))?;
    compare_rows(table.file(), expected, &actual)
}

pub(super) fn verify_snapshots(dir: &Path, corpus: &crate::Corpus) -> Result<(), String> {
    compare_table(dir, Table::Schools, &corpus.schools)?;
    compare_table(dir, Table::Teams, &corpus.teams)?;
    compare_table(dir, Table::Athletes, &corpus.athletes)?;
    compare_table(dir, Table::Meets, &corpus.meets)?;
    compare_table(dir, Table::Events, &corpus.events)?;
    compare_table(dir, Table::Performances, &corpus.performances)?;
    Ok(())
}

pub(super) fn verify_physical_observations(
    store: &Store,
    corpus: &crate::Corpus,
) -> Result<(), String> {
    let snapshot = store.snapshot();
    let mut schools = Vec::new();
    snapshot
        .for_each_observation(Table::Schools, |row: CanonicalSchool| {
            schools.push(row);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    compare_rows(Table::Schools.file(), &corpus.schools, &schools)?;
    let mut teams = Vec::new();
    snapshot
        .for_each_observation(Table::Teams, |row: CanonicalTeam| {
            teams.push(row);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    compare_rows(Table::Teams.file(), &corpus.teams, &teams)?;
    let mut athletes = Vec::new();
    snapshot
        .for_each_observation(Table::Athletes, |row: CanonicalAthlete| {
            athletes.push(row);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    compare_rows(Table::Athletes.file(), &corpus.athletes, &athletes)?;
    let mut meets = Vec::new();
    snapshot
        .for_each_observation(Table::Meets, |row: CanonicalMeet| {
            meets.push(row);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    compare_rows(Table::Meets.file(), &corpus.meets, &meets)?;
    let mut events = Vec::new();
    snapshot
        .for_each_observation(Table::Events, |row: CanonicalEvent| {
            events.push(row);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    compare_rows(Table::Events.file(), &corpus.events, &events)?;
    let mut performances = Vec::new();
    snapshot
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            performances.push(row);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    compare_rows(Table::Performances.file(), &corpus.performances, &performances)?;
    let stats = store.stats().map_err(|error| error.to_string())?;
    let expected = corpus.appended_rows() as u64;
    if stats.observations != expected {
        return Err(format!(
            "the store holds {} physical observations, not the corpus's {expected}",
            stats.observations
        ));
    }
    Ok(())
}

pub(super) fn verify_reply_tables(reply: &Value, dir: &Path) -> Result<(), String> {
    let tables = reply
        .get("tables")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the workflow reply holds no tables array: {reply}"))?;
    let mut seen = BTreeMap::new();
    for entry in tables {
        let name = entry
            .get("table")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("a reply entry names no table: {entry}"))?;
        let rows = entry
            .get("rows")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("reply entry {name} holds no row count: {entry}"))?;
        let table = Table::from_wire(name)
            .ok_or_else(|| format!("the reply names unknown table {name}"))?;
        if seen.insert(name.to_string(), rows).is_some() {
            return Err(format!("the reply names table {name} more than once"));
        }
        let observed = snapshot_line_count(&snapshot_file(dir, table))?;
        if observed as u64 != rows {
            return Err(format!(
                "the reply reports {rows} rows for {name} but its snapshot holds {observed}"
            ));
        }
    }
    for table in Table::ALL {
        if !seen.contains_key(table.file()) {
            return Err(format!("the reply omits table {}", table.file()));
        }
    }
    Ok(())
}

pub(super) fn require_reply_body(body: &Value) -> Result<(), String> {
    match body.get("tables").and_then(Value::as_array) {
        Some(_) => Ok(()),
        None => Err(format!(
            "the attached output is not a consolidation reply: {body}"
        )),
    }
}

pub(super) fn single_invocation_row<'a>(
    rows: &'a [Value],
    expected_id: &str,
) -> Result<&'a Value, String> {
    match rows {
        [row] => {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("the invocation answer holds no id: {row}"))?;
            if id == expected_id {
                Ok(row)
            } else {
                Err(format!(
                    "the invocation answer identifies {id}, not {expected_id}"
                ))
            }
        }
        other => Err(format!(
            "the invocation answer holds {} rows for {expected_id}, not one",
            other.len()
        )),
    }
}

pub(super) fn invocation_verdict(row: &Value, expected_id: &str) -> Verdict {
    let id = row.get("id").and_then(Value::as_str);
    if id != Some(expected_id) {
        return Verdict::Refused(format!(
            "the observation identifies {id:?}, not {expected_id}"
        ));
    }
    match row.get("status").and_then(Value::as_str) {
        Some("completed") => match row.get("completion_result").and_then(Value::as_str) {
            Some("success") => Verdict::Succeeded,
            Some("failure") => Verdict::Refused(format!(
                "completed with failure: {}",
                row.get("completion_failure").cloned().unwrap_or(Value::Null)
            )),
            other => Verdict::Refused(format!("completed with completion_result {other:?}")),
        },
        Some("paused") => Verdict::Refused(String::from("the invocation is still paused")),
        Some("killed") => Verdict::Refused(String::from("the invocation was killed")),
        Some(other @ ("pending" | "scheduled" | "ready" | "running" | "backing-off" | "suspended")) => {
            Verdict::Pending(format!("the invocation is still {other}"))
        }
        other => Verdict::Refused(format!("the invocation reported status {other:?}")),
    }
}
