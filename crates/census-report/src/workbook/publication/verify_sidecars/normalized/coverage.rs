use super::super::input::Inputs;
use super::{
    rows::{self, Row},
    schema,
};
use crate::report::ReportResult;
use census_domain::model::{CoachLaneObligation, CoverageRow, SourceAccessCondition};
use std::collections::BTreeMap;
use std::path::Path;

enum Coverage<'a> {
    Metrics(&'a CoverageRow),
    Coach(&'a CoachLaneObligation),
    Access(&'a SourceAccessCondition),
}

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<u64> {
    let dataset = inputs.dataset;
    let metrics = dataset
        .coverage_evidence
        .rows
        .iter()
        .map(|fact| (format!("metrics:{}", fact.id), Coverage::Metrics(fact)));
    let coaches = dataset
        .coverage_evidence
        .coach_lanes
        .iter()
        .map(|fact| (format!("coach_lane:{}", fact.id()), Coverage::Coach(fact)));
    let access = dataset
        .source_access
        .iter()
        .map(|fact| (format!("source_access:{}", fact.id), Coverage::Access(fact)));
    let inventory: BTreeMap<_, _> = metrics.chain(coaches).chain(access).collect();
    rows::table(
        directory,
        ("coverage.csv", schema::COVERAGE),
        inventory.len(),
        &["coverage_id"],
        |row| {
            let id = row.get("coverage_id")?;
            let fact = inventory
                .get(id)
                .ok_or_else(|| rows::missing("coverage", id))?;
            match fact {
                Coverage::Metrics(fact) => metric(row, fact)?,
                Coverage::Coach(fact) => coach(row, fact)?,
                Coverage::Access(fact) => source(row, fact)?,
            }
            row.text(
                "cohort",
                dataset
                    .lineage
                    .cohort
                    .map_or_else(|| "unavailable".into(), |year| year.get().to_string()),
            )?;
            row.text("input_generation", &dataset.lineage.input_generation)?;
            row.literals(&[
                ("sport", "unavailable"),
                ("category", "unavailable"),
                ("discovery_state", "unavailable"),
                ("last_successful_refresh", "unavailable"),
                ("excluded_uncertain_counts", "unavailable"),
                ("denominator", "unavailable_known_universe_not_retained"),
            ])
        },
    )
}

fn metric(row: &Row<'_>, fact: &CoverageRow) -> ReportResult<()> {
    row.text("coverage_scope", fact.scope.slug())?;
    row.text("subject", &fact.subject)?;
    row.json("observed_metrics", &fact.metrics)?;
    row.json("retained_record", fact)?;
    row.text(
        "evidence_state",
        if fact.metrics.is_empty() {
            "unmeasured"
        } else {
            "retained_metrics_scope_not_qualified"
        },
    )?;
    row.literals(&[
        ("source_scope_searched", "unavailable"),
        ("fetch_state", "unavailable"),
        ("unresolved_gaps", "scope_gap_metadata_not_retained"),
    ])
}

fn coach(row: &Row<'_>, fact: &CoachLaneObligation) -> ReportResult<()> {
    row.text("coverage_scope", "coach_lane")?;
    row.text("subject", fact.jurisdiction().code())?;
    row.text("source_scope_searched", fact.arm_name())?;
    row.text("evidence_state", fact.status().slug())?;
    row.text(
        "unresolved_gaps",
        rows::available(fact.refusal().map(|refusal| refusal.reason())),
    )?;
    match fact.evidence() {
        Some(evidence) => row.json(
            "observed_metrics",
            &BTreeMap::from([
                ("observed_schools", evidence.schools()),
                ("observed_contacts", evidence.contacts()),
            ]),
        )?,
        None => row.text("observed_metrics", "unavailable")?,
    }
    row.json("retained_record", fact)?;
    row.text("fetch_state", "unavailable")
}

fn source(row: &Row<'_>, fact: &SourceAccessCondition) -> ReportResult<()> {
    row.text("coverage_scope", "source_access")?;
    row.text("subject", &fact.source)?;
    row.text("fetch_state", fact.kind.slug())?;
    row.text("unresolved_gaps", rows::available(Some(&fact.detail)))?;
    row.json("retained_record", fact)?;
    row.literals(&[
        ("source_scope_searched", "unavailable"),
        ("observed_metrics", "unavailable"),
        ("evidence_state", "retained_access_failure"),
    ])
}
