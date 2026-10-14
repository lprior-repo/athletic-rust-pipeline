use super::{digest, encode, write_csv};
use crate::bests::{Measure, SharedSelection};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use std::collections::BTreeMap;
use std::path::Path;

const HEADER: [&str; 18] = [
    "comparison_class_id",
    "event_identity",
    "sport",
    "category",
    "ranking_direction",
    "normalized_unit",
    "precision",
    "condition_requirements",
    "comparability_label",
    "surface",
    "policy_version",
    "specification",
    "context_event_id",
    "adjustment_class",
    "catalog_digest",
    "input_generation",
    "precision_basis",
    "allowed_result_statuses",
];

pub(super) fn id(selection: &SharedSelection) -> ReportResult<String> {
    let key = &selection.key;
    digest(&(
        &key.event_kind,
        key.surface,
        key.wind_class,
        key.timing,
        key.measure,
        &key.context,
        &key.specification,
        key.comparison,
        key.adjustment,
        selection.athlete.gender,
    ))
}

pub(super) fn write(
    directory: &Path,
    dataset: &ExportDataset,
    bests: &[SharedSelection],
) -> ReportResult<()> {
    let catalog = census_domain::model::RecruitingCatalogV1::new()
        .digest()
        .map_err(|error| super::defect(format!("encoding catalog: {error}")))?;
    let mut classes = BTreeMap::new();
    let mut budget = crate::grouping::Admission::new("normalized comparison classes");
    for best in bests {
        let id = id(best)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = classes.entry(id) {
            budget.record(entry.key())?;
            entry.insert(best);
        }
    }
    write_csv(
        &directory.join("comparison_classes.csv"),
        &HEADER,
        |writer| {
            for (id, best) in classes {
                writer.push(row(id, best, dataset, &catalog)?)?;
            }
            Ok(())
        },
    )
}

fn row(
    id: String,
    best: &SharedSelection,
    dataset: &ExportDataset,
    catalog: &str,
) -> ReportResult<[String; 18]> {
    let key = &best.key;
    Ok([
        id,
        best.event_key(),
        best.sport().into(),
        best.athlete.gender.stable_key().into(),
        match key.measure {
            Measure::Time => "lower_is_better",
            Measure::Distance | Measure::Points => "higher_is_better",
        }
        .into(),
        key.measure.value_unit().into(),
        "1".into(),
        encode(&(key.timing, key.wind_class, &key.specification))?,
        encode(&key.comparison)?,
        key.surface.label().into(),
        dataset.lineage.policy_revision.to_string(),
        encode(&key.specification)?,
        key.context
            .as_ref()
            .map_or_else(|| "not_applicable".into(), ToString::to_string),
        encode(&key.adjustment)?,
        catalog.into(),
        dataset.lineage.input_generation.clone(),
        "normalized_integer_quantum".into(),
        "unavailable_result_status_schema_not_retained".into(),
    ])
}
