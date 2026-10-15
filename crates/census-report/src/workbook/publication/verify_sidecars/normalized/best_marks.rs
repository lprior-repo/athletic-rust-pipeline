use super::super::input::Inputs;
use super::revision::Revision;
use super::{
    classes, facts,
    rows::{self, Row},
};
use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::{CanonicalPerformance, Mark};
use std::collections::BTreeMap;
use std::path::Path;

type Performances<'a> = BTreeMap<&'a str, &'a CanonicalPerformance>;

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    bests: &[SharedSelection],
    revision: Revision,
) -> ReportResult<u64> {
    let expected = bests
        .iter()
        .map(|best| Ok(((best.athlete_id().to_string(), classes::id(best)?), best)))
        .collect::<ReportResult<BTreeMap<_, _>>>()?;
    let performances: Performances<'_> = inputs
        .derivation
        .performances()
        .iter()
        .map(|fact| (fact.id.as_str(), *fact))
        .collect();
    rows::table(
        directory,
        ("best_marks.csv", &revision.best_headers()),
        expected.len(),
        &["athlete_id", "comparison_class_id"],
        |row| {
            let key = (
                row.get("athlete_id")?.to_owned(),
                row.get("comparison_class_id")?.to_owned(),
            );
            let best = expected
                .get(&key)
                .ok_or_else(|| rows::missing("best key", &format!("{key:?}")))?;
            let performance_id = row.get("performance_id")?;
            let performance = performances
                .get(performance_id)
                .copied()
                .ok_or_else(|| rows::missing("performance", performance_id))?;
            selected(row, inputs, best, performance)?;
            inventory(row, best, &performances, revision)?;
            source(row, inputs, performance)?;
            row.number(
                "selection_policy_version",
                inputs.dataset.lineage.policy_revision,
            )?;
            row.text("input_generation", &inputs.dataset.lineage.input_generation)
        },
    )
}

fn selected(
    row: &Row<'_>,
    inputs: &Inputs<'_>,
    best: &SharedSelection,
    performance: &CanonicalPerformance,
) -> ReportResult<()> {
    row.text("performance_id", best.source.performance_id.as_str())?;
    let identities = inputs.dataset.identities();
    let canonical = identities.canonical_id(performance.athlete.as_str());
    if canonical != best.athlete_id().as_str()
        || inputs.admission.included_kind(&performance.event).is_none()
    {
        return Err(rows::missing(
            "admitted athlete/event",
            performance.id.as_str(),
        ));
    }
    let value = best
        .measure()
        .value(&performance.mark)
        .ok_or_else(|| rows::missing("normalized mark", performance.id.as_str()))?;
    if value != best.result.value {
        return Err(rows::missing("selected value", performance.id.as_str()));
    }
    row.number("normalized_mark", value)?;
    row.text("display_mark", crate::bests::mark_text(&performance.mark))?;
    row.text(
        "raw_mark",
        match &performance.mark {
            Mark::Raw(value) => value,
            Mark::FieldImperial { feet_mark, .. } => feet_mark,
            _ => "unavailable",
        },
    )?;
    row.text("unit", best.measure().value_unit())?;
    row.number("precision", 1)?;
    row.text(
        "source_decimal_precision",
        match &performance.mark {
            Mark::TimeSeconds(time) => time.precision().to_string(),
            _ => "unavailable".into(),
        },
    )?;
    row.text("event_label", best.event_label())?;
    row.text("timing_class", best.key.timing.label())?;
    row.text("wind_class", best.key.wind_class.label())?;
    row.text("surface", best.key.surface.label())?;
    let event = inputs
        .admission
        .event(&performance.event)
        .ok_or_else(|| rows::missing("event", performance.event.as_str()))?;
    let specification = event
        .resolved_specification()
        .map_err(|error| super::super::defect(error.to_string()))?;
    row.json("technical_class", &specification)?;
    course(row, &specification)
}

fn course(
    row: &Row<'_>,
    specification: &census_domain::model::EventSpecification,
) -> ReportResult<()> {
    match &specification.cross_country {
        Some(context) => match &context.course {
            Some(course) => row.json("course", course),
            None => row.text("course", "unavailable"),
        },
        None => row.text("course", "not_applicable"),
    }
}

fn inventory(
    row: &Row<'_>,
    best: &SharedSelection,
    performances: &Performances<'_>,
    revision: Revision,
) -> ReportResult<()> {
    let (eligible, tied, ids) = if matches!(revision, Revision::ResultGroups) {
        row.text("result_group_basis", "canonical_performance_id")?;
        row.text("physical_performance_count_status", "unknown")?;
        (
            "eligible_result_group_count",
            "tied_result_group_count",
            "tied_result_group_representative_ids",
        )
    } else {
        (
            "eligible_candidate_count",
            "tied_performance_count",
            "tied_performance_ids",
        )
    };
    row.number(eligible, best.inventory.eligible_group_count())?;
    row.number(tied, best.inventory.tied_group_count())?;
    row.json(ids, &best.inventory.tied_representative_ids())?;
    if revision.has_supporting() {
        row.json(
            "supporting_tied_candidate_ids",
            &best.supporting_tied_candidate_ids(),
        )?;
    }
    revision.ids(best).iter().try_for_each(|id| {
        let performance = performances
            .get(id.as_str())
            .ok_or_else(|| rows::missing("tied performance", id.as_str()))?;
        if best.measure().value(&performance.mark) == Some(best.result.value) {
            Ok(())
        } else {
            Err(rows::missing("tied mark", id.as_str()))
        }
    })
}

fn source(
    row: &Row<'_>,
    inputs: &Inputs<'_>,
    performance: &CanonicalPerformance,
) -> ReportResult<()> {
    let meet = inputs
        .dataset
        .meets
        .iter()
        .find(|meet| meet.id == performance.meet)
        .ok_or_else(|| rows::missing("meet", performance.meet.as_str()))?;
    row.text("meet_id", performance.meet.as_str())?;
    row.text("meet_date", rows::available(Some(&performance.date)))?;
    row.text("meet_name", rows::available(Some(&meet.name)))?;
    let school = inputs
        .dataset
        .teams
        .get(&performance.team)
        .map(|team| team.school.as_str());
    row.text("school_at_performance", rows::available(school))?;
    row.text(
        "school_at_performance_status",
        if school.is_some() {
            "performance_team_school"
        } else {
            "team_school_unavailable"
        },
    )?;
    source_url_row(row, inputs, performance)?;
    evidence_row(row, performance)?;
    source_literals(row)
}

fn source_url_row(
    row: &Row<'_>,
    inputs: &Inputs<'_>,
    performance: &CanonicalPerformance,
) -> ReportResult<()> {
    let url = inputs
        .derivation
        .scope()
        .primary_evidence(performance)
        .and_then(|fact| fact.source.url.as_deref());
    row.text("source_url", rows::available(url))?;
    row.text(
        "source_url_status",
        if url.is_some_and(|url| !url.is_empty()) {
            "retained_result_locator"
        } else {
            "source_locator_unavailable"
        },
    )
}

fn evidence_row(row: &Row<'_>, performance: &CanonicalPerformance) -> ReportResult<()> {
    row.json(
        "evidence_refs",
        &facts::references(
            "performance",
            performance.id.as_str(),
            &performance.evidence,
        )?,
    )?;
    row.text(
        "result_evidence_status",
        if performance.evidence.is_empty() {
            "no_retained_result_evidence"
        } else {
            "retained_evidence_capture_links_unavailable"
        },
    )
}

fn source_literals(row: &Row<'_>) -> ReportResult<()> {
    row.literals(&[
        ("performance_version", "unavailable"),
        (
            "performance_version_status",
            "immutable_performance_version_not_retained",
        ),
        (
            "raw_mark_status",
            "source_text_only_retained_for_raw_or_imperial_marks",
        ),
        ("capture_reference", "unavailable"),
        ("capture_reference_status", "capture_link_not_retained"),
        ("precision_basis", "normalized_integer_quantum"),
    ])
}
