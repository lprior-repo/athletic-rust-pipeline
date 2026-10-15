use super::{SelectionOutcome, SharedSelection};
use crate::export::ExportDataset;
use crate::report::{Derivation, ReportError, ReportResult};
use census_store::read::csv_failure;
use census_store::{StoreError, StoreResult};
use std::path::{Path, PathBuf};

mod row;
pub(super) mod selected;
use row::Row;

const HEADER: [&str; 36] = [
    "athlete_id",
    "name",
    "school",
    "state",
    "grad_year",
    "gender",
    "sport",
    "event",
    "surface",
    "wind_class",
    "timing_class",
    "best_mark",
    "original_mark",
    "best_value",
    "best_value_unit",
    "measure",
    "date",
    "meet",
    "place",
    "wind_mps",
    "timing",
    "marks_in_event",
    "profile_url",
    "performance_id",
    "source_athlete",
    "source_key",
    "comparison_policy",
    "comparison_specification",
    "source_specification",
    "adjustment_kind",
    "adjustment_basis",
    "adjustment_note",
    "adjustment_source",
    "eligible_candidate_count",
    "tied_performance_count",
    "tied_performance_ids",
];

pub fn write(
    out_dir: &Path,
    rows: &[SharedSelection],
    cohort: &str,
    dataset: &ExportDataset,
) -> ReportResult<(PathBuf, PathBuf)> {
    if dataset.selected_profile_id().is_some() {
        return Err(ReportError::Invariant {
            detail: "selected participation policy requires verified frozen evidence".into(),
        });
    }
    super::participation::ParticipationSelection::of(dataset)?.ensure_dataset(dataset)?;
    write_rows(out_dir, rows, cohort, dataset.semantic_policy()?).map_err(ReportError::from)
}

pub(crate) fn write_outcome(
    out_dir: &Path,
    derivation: &Derivation<'_>,
    outcome: &SelectionOutcome,
    cohort: &str,
) -> ReportResult<(PathBuf, PathBuf)> {
    let dataset = derivation.dataset();
    derivation.ensure_selected_affiliation()?;
    super::participation::ParticipationSelection::of(dataset)?.ensure_dataset(dataset)?;
    if dataset.selected_profile_id().is_some() {
        dataset.verified_participation()?;
        dataset.verified_current_affiliations()?;
        return selected::write(
            out_dir,
            &outcome.selections,
            cohort,
            dataset.semantic_policy()?,
            dataset,
        );
    }
    write_rows(
        out_dir,
        &outcome.selections,
        cohort,
        dataset.semantic_policy()?,
    )
    .map_err(ReportError::from)
}

fn write_rows(
    out_dir: &Path,
    rows: &[SharedSelection],
    cohort: &str,
    policy: census_domain::model::EventSemantics,
) -> StoreResult<(PathBuf, PathBuf)> {
    if rows.iter().any(|row| row.inventory.policy() != policy) {
        return Err(StoreError::Invariant {
            detail: "best-results inventory differs from the explicit selection policy".into(),
        });
    }
    census_store::fs::create_dir_all_synced(out_dir).map_err(|source| StoreError::Io {
        path: out_dir.to_path_buf(),
        source,
    })?;
    let jsonl = out_dir.join(format!("best-results-{cohort}.jsonl"));
    let csv_path = out_dir.join(format!("best-results-{cohort}.csv"));
    census_store::read::write_snapshot_rows(&jsonl, rows)?;
    write_csv(&csv_path, rows, policy)?;
    Ok((jsonl, csv_path))
}

fn write_csv(
    path: &Path,
    rows: &[SharedSelection],
    policy: census_domain::model::EventSemantics,
) -> StoreResult<()> {
    census_store::read::publish_atomically(path, |temporary| {
        write_csv_body(temporary, rows, policy)
    })
}

fn write_csv_body(
    temporary: &Path,
    rows: &[SharedSelection],
    policy: census_domain::model::EventSemantics,
) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(temporary)
        .map_err(|error| csv_failure(temporary, error))?;
    writer
        .write_record(header(policy))
        .map_err(|error| csv_failure(temporary, error))?;
    rows.iter().try_for_each(|row| {
        let csv_row: Row<'_> = row.into();
        writer
            .serialize(&csv_row)
            .map_err(|error| csv_failure(temporary, error))
    })?;
    writer.flush().map_err(|source| StoreError::Io {
        path: temporary.to_path_buf(),
        source,
    })
}

fn header(policy: census_domain::model::EventSemantics) -> Vec<&'static str> {
    if policy != census_domain::model::EventSemantics::ResultGroups21 {
        return HEADER.to_vec();
    }
    HEADER
        .iter()
        .take(33)
        .copied()
        .chain([
            "eligible_result_group_count",
            "tied_result_group_count",
            "tied_result_group_representative_ids",
            "result_group_basis",
            "physical_performance_count_status",
            "selection_policy_version",
        ])
        .collect()
}

#[cfg(test)]
pub(super) mod tests;
