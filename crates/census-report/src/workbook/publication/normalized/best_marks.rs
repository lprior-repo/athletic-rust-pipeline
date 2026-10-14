use super::{comparison_classes, encode, provenance, text, write_csv};
use crate::bests::SharedSelection;
use crate::report::{Derivation, ReportResult};
use std::path::Path;

const HEADER: [&str; 35] = [
    "athlete_id",
    "comparison_class_id",
    "performance_id",
    "performance_version",
    "performance_version_status",
    "event_label",
    "raw_mark",
    "raw_mark_status",
    "display_mark",
    "normalized_mark",
    "unit",
    "precision",
    "timing_class",
    "wind_class",
    "technical_class",
    "meet_id",
    "meet_date",
    "meet_name",
    "course",
    "school_at_performance",
    "school_at_performance_status",
    "source_url",
    "capture_reference",
    "capture_reference_status",
    "evidence_refs",
    "tied_performance_count",
    "eligible_candidate_count",
    "tied_performance_ids",
    "selection_policy_version",
    "surface",
    "input_generation",
    "precision_basis",
    "source_decimal_precision",
    "result_evidence_status",
    "source_url_status",
];

pub(super) fn write(
    directory: &Path,
    derivation: &Derivation<'_>,
    bests: &[SharedSelection],
    performances: &super::PerformanceIndex<'_>,
) -> ReportResult<()> {
    write_csv(&directory.join("best_marks.csv"), &HEADER, |writer| {
        for best in bests {
            writer.push(row(best, derivation, performances)?)?;
        }
        Ok(())
    })
}

fn row(
    best: &SharedSelection,
    derivation: &Derivation<'_>,
    performances: &super::PerformanceIndex<'_>,
) -> ReportResult<[String; 35]> {
    let dataset = derivation.dataset();
    let performance = performances
        .get(&best.source.performance_id)
        .copied()
        .ok_or(super::defect(format!(
            "missing selected performance {}",
            best.source.performance_id
        )))?;
    let school = dataset
        .teams
        .get(&performance.team)
        .map(|team| team.school.as_str());
    Ok([
        best.athlete_id().to_string(),
        comparison_classes::id(best)?,
        best.source.performance_id.to_string(),
        "unavailable".into(),
        "immutable_performance_version_not_retained".into(),
        best.event_label(),
        raw_mark_value(&performance.mark),
        "source_text_only_retained_for_raw_or_imperial_marks".into(),
        best.mark_text(),
        best.result.value.to_string(),
        best.measure().value_unit().into(),
        "1".into(),
        best.key.timing.label().into(),
        best.key.wind_class.label().into(),
        encode(&best.source.specification)?,
        best.meet.meet_id.to_string(),
        text(Some(&performance.date)),
        text(Some(&best.meet.name)),
        course(best)?,
        text(school),
        school_status(school),
        text(Some(&best.source.result_url)),
        "unavailable".into(),
        "capture_link_not_retained".into(),
        provenance::references(
            "performance",
            performance.id.as_str(),
            &performance.evidence,
        )?,
        best.tied_performance_count.to_string(),
        best.eligible_candidate_count.to_string(),
        encode(&best.tied_performance_ids)?,
        dataset.lineage.policy_revision.to_string(),
        best.key.surface.label().into(),
        dataset.lineage.input_generation.clone(),
        "normalized_integer_quantum".into(),
        source_decimal_precision(&performance.mark),
        evidence_status(&performance.evidence),
        source_url_status(&best.source.result_url),
    ])
}

fn raw_mark_value(mark: &census_domain::model::Mark) -> String {
    match mark {
        census_domain::model::Mark::Raw(value) => value.clone(),
        census_domain::model::Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
        _ => "unavailable".into(),
    }
}

fn school_status(school: Option<&str>) -> String {
    if school.is_some() {
        "performance_team_school"
    } else {
        "team_school_unavailable"
    }
    .into()
}

fn source_url_status(url: &str) -> String {
    if url.is_empty() {
        "source_locator_unavailable"
    } else {
        "retained_result_locator"
    }
    .into()
}

fn evidence_status(evidence: &[census_domain::model::Evidence]) -> String {
    if evidence.is_empty() {
        "no_retained_result_evidence"
    } else {
        "retained_evidence_capture_links_unavailable"
    }
    .into()
}

fn source_decimal_precision(mark: &census_domain::model::Mark) -> String {
    match mark {
        census_domain::model::Mark::TimeSeconds(time) => time.precision().to_string(),
        _ => "unavailable".into(),
    }
}

fn course(best: &SharedSelection) -> ReportResult<String> {
    match &best.source.specification.cross_country {
        Some(context) => match &context.course {
            Some(course) => encode(course),
            None => Ok("unavailable".into()),
        },
        None => Ok("not_applicable".into()),
    }
}
