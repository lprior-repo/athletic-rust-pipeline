use super::super::input::Inputs;
use super::{
    rows::{self, Row},
    schema,
};
use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::performance_comparison::Measure;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn id(best: &SharedSelection) -> ReportResult<String> {
    let key = &best.key;
    rows::digest(&(
        &key.event_kind,
        key.surface,
        key.wind_class,
        key.timing,
        key.measure,
        &key.context,
        &key.specification,
        key.comparison,
        key.adjustment,
        best.athlete.gender,
    ))
}

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    bests: &[SharedSelection],
) -> ReportResult<()> {
    let classes =
        bests
            .iter()
            .try_fold(BTreeMap::new(), |mut classes, best| -> ReportResult<_> {
                classes.insert(id(best)?, best);
                Ok(classes)
            })?;
    let catalog = census_domain::model::RecruitingCatalogV1::new()
        .digest()
        .map_err(|error| super::super::defect(error.to_string()))?;
    rows::table(
        directory,
        ("comparison_classes.csv", schema::CLASSES),
        classes.len(),
        &["comparison_class_id"],
        |row| {
            let id = row.get("comparison_class_id")?;
            let best = classes
                .get(id)
                .ok_or_else(|| rows::missing("comparison class", id))?;
            attributes(row, best)?;
            row.number("policy_version", inputs.dataset.lineage.policy_revision)?;
            row.text("catalog_digest", &catalog)?;
            row.text("input_generation", &inputs.dataset.lineage.input_generation)
        },
    )
}

fn attributes(row: &Row<'_>, best: &SharedSelection) -> ReportResult<()> {
    let key = &best.key;
    row.text("event_identity", best.event_key())?;
    row.text("sport", best.sport())?;
    row.text("category", best.athlete.gender.stable_key())?;
    row.text(
        "ranking_direction",
        match key.measure {
            Measure::Time => "lower_is_better",
            Measure::Distance | Measure::Points => "higher_is_better",
        },
    )?;
    row.text("normalized_unit", key.measure.value_unit())?;
    row.number("precision", 1)?;
    row.json(
        "condition_requirements",
        &(key.timing, key.wind_class, &key.specification),
    )?;
    row.json("comparability_label", &key.comparison)?;
    row.text("surface", key.surface.label())?;
    row.json("specification", &key.specification)?;
    row.text(
        "context_event_id",
        key.context
            .as_ref()
            .map_or_else(|| "not_applicable".into(), ToString::to_string),
    )?;
    row.json("adjustment_class", &key.adjustment)?;
    row.literals(&[
        ("precision_basis", "normalized_integer_quantum"),
        (
            "allowed_result_statuses",
            "unavailable_result_status_schema_not_retained",
        ),
    ])
}
