use super::{encode, text, write_csv};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{CoachLaneObligation, CoverageRow, SourceAccessCondition};
use std::path::Path;

const HEADER: [&str; 17] = [
    "coverage_id",
    "coverage_scope",
    "subject",
    "cohort",
    "sport",
    "category",
    "source_scope_searched",
    "discovery_state",
    "fetch_state",
    "last_successful_refresh",
    "observed_metrics",
    "excluded_uncertain_counts",
    "unresolved_gaps",
    "denominator",
    "evidence_state",
    "input_generation",
    "retained_record",
];

pub(super) fn write(directory: &Path, dataset: &ExportDataset) -> ReportResult<()> {
    write_csv(&directory.join("coverage.csv"), &HEADER, |writer| {
        for coverage in &dataset.coverage_evidence.rows {
            writer.push(metrics(dataset, coverage)?)?;
        }
        for lane in &dataset.coverage_evidence.coach_lanes {
            writer.push(coach_lane(dataset, lane)?)?;
        }
        for access in &dataset.source_access {
            writer.push(source_access(dataset, access)?)?;
        }
        Ok(())
    })
}

fn cohort(dataset: &ExportDataset) -> String {
    dataset
        .lineage
        .cohort
        .map_or_else(|| "unavailable".into(), |year| year.get().to_string())
}

fn metrics(dataset: &ExportDataset, row: &CoverageRow) -> ReportResult<[String; 17]> {
    Ok([
        format!("metrics:{}", row.id),
        row.scope.slug().into(),
        row.subject.clone(),
        cohort(dataset),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        encode(&row.metrics)?,
        "unavailable".into(),
        "scope_gap_metadata_not_retained".into(),
        "unavailable_known_universe_not_retained".into(),
        if row.metrics.is_empty() {
            "unmeasured"
        } else {
            "retained_metrics_scope_not_qualified"
        }
        .into(),
        dataset.lineage.input_generation.clone(),
        encode(row)?,
    ])
}

fn coach_lane(dataset: &ExportDataset, lane: &CoachLaneObligation) -> ReportResult<[String; 17]> {
    let metrics = match lane.evidence() {
        Some(evidence) => encode(&std::collections::BTreeMap::from([
            ("observed_schools", evidence.schools()),
            ("observed_contacts", evidence.contacts()),
        ]))?,
        None => "unavailable".into(),
    };
    Ok([
        format!("coach_lane:{}", lane.id()),
        "coach_lane".into(),
        lane.jurisdiction().code().into(),
        cohort(dataset),
        "unavailable".into(),
        "unavailable".into(),
        lane.arm_name().into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        metrics,
        "unavailable".into(),
        text(lane.refusal().map(|refusal| refusal.reason())),
        "unavailable_known_universe_not_retained".into(),
        lane.status().slug().into(),
        dataset.lineage.input_generation.clone(),
        encode(lane)?,
    ])
}

fn source_access(
    dataset: &ExportDataset,
    access: &SourceAccessCondition,
) -> ReportResult<[String; 17]> {
    Ok([
        format!("source_access:{}", access.id),
        "source_access".into(),
        access.source.clone(),
        cohort(dataset),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        access.kind.slug().into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        text(Some(&access.detail)),
        "unavailable_known_universe_not_retained".into(),
        "retained_access_failure".into(),
        dataset.lineage.input_generation.clone(),
        encode(access)?,
    ])
}
