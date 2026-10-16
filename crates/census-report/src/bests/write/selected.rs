use super::{header, Row, SharedSelection};
use crate::export::current_school::{CurrentSchoolFields, SelectedSchools};
use crate::export::public_identity::output::AcceptedIdentityFields;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::EventSemantics;
use census_store::read::{csv_failure, publish_atomically};
use census_store::{StoreError, StoreResult};
use serde::Serialize;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const MAX_SELECTED_ROWS: usize = 4_000_000;

#[derive(Serialize)]
struct SelectedIdentitySelection<'a> {
    schema: &'static str,
    #[serde(flatten)]
    identity: AcceptedIdentityFields<'a>,
    #[serde(flatten)]
    current: CurrentSchoolFields<'a>,
    selection: &'a SharedSelection,
}

pub(crate) fn write(
    directory: &Path,
    selections: &[SharedSelection],
    cohort: &str,
    policy: EventSemantics,
    dataset: &ExportDataset,
) -> ReportResult<(PathBuf, PathBuf)> {
    dataset.ensure_current_selected_write()?;
    let schools = SelectedSchools::of(dataset)?;
    preflight(selections, policy, &schools)?;
    census_store::fs::create_dir_all_synced(directory).map_err(|source| io(directory, source))?;
    let jsonl = directory.join(format!("best-results-{cohort}.jsonl"));
    let csv = directory.join(format!("best-results-{cohort}.csv"));
    publish_atomically(&jsonl, |temporary| {
        jsonl_body(temporary, selections, &schools)
    })?;
    publish_atomically(&csv, |temporary| {
        csv_body(temporary, selections, policy, &schools)
    })?;
    Ok((jsonl, csv))
}

fn preflight(
    selections: &[SharedSelection],
    policy: EventSemantics,
    schools: &SelectedSchools<'_>,
) -> StoreResult<()> {
    if policy != EventSemantics::ResultGroups21 {
        return Err(StoreError::Invariant {
            detail: "selected public identity output requires result-group policy 21".into(),
        });
    }
    if selections.len() > MAX_SELECTED_ROWS {
        return Err(StoreError::Invariant {
            detail: "selected public identity output exceeds 4000000 rows".into(),
        });
    }
    if selections
        .iter()
        .any(|row| row.inventory.policy() != policy)
    {
        return Err(StoreError::Invariant {
            detail: "best-results inventory differs from the explicit selection policy".into(),
        });
    }
    selections
        .iter()
        .try_for_each(|row| schools.fields(row.athlete_id().as_str()).map(|_| ()))
}

fn jsonl_body(
    path: &Path,
    selections: &[SharedSelection],
    schools: &SelectedSchools<'_>,
) -> StoreResult<()> {
    let file = std::fs::File::create(path).map_err(|source| io(path, source))?;
    let mut writer = BufWriter::new(file);
    selections.iter().try_for_each(|selection| {
        let row = SelectedIdentitySelection {
            schema: "selected_identity_v2",
            identity: schools.identity(selection.athlete_id().as_str())?,
            current: schools.fields(selection.athlete_id().as_str())?,
            selection,
        };
        serde_json::to_writer(&mut writer, &row).map_err(|source| StoreError::Json {
            detail: "writing selected public identity best-results".into(),
            source,
        })?;
        writer.write_all(b"\n").map_err(|source| io(path, source))
    })?;
    writer.flush().map_err(|source| io(path, source))
}

fn csv_body(
    path: &Path,
    selections: &[SharedSelection],
    policy: EventSemantics,
    schools: &SelectedSchools<'_>,
) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(path)
        .map_err(|error| csv_failure(path, error))?;
    writer
        .write_record(selected_header(policy))
        .map_err(|error| csv_failure(path, error))?;
    selections.iter().try_for_each(|selection| {
        let identity = schools.identity(selection.athlete_id().as_str())?;
        writer
            .serialize(Row::selected(
                selection,
                identity,
                schools.fields(selection.athlete_id().as_str())?,
            ))
            .map_err(|error| csv_failure(path, error))
    })?;
    writer.flush().map_err(|source| io(path, source))
}

fn io(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn selected_header(policy: EventSemantics) -> Vec<&'static str> {
    header(policy)
        .into_iter()
        .map(|name| match name {
            "school" => "school_at_performance",
            "state" => "state_at_performance",
            other => other,
        })
        .chain([
            "canonical_candidate_id",
            "current_school_id",
            "current_school",
            "current_state",
            "current_school_as_of",
        ])
        .collect()
}
