use super::super::input::Inputs;
use super::{
    facts,
    rows::{self, Row},
    schema,
};
use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, Confidence, Gender};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    bests: &[SharedSelection],
) -> ReportResult<()> {
    let athletes: BTreeMap<_, _> = inputs
        .derivation
        .athletes()
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect();
    let marked: BTreeSet<_> = bests.iter().map(|row| row.athlete_id().as_str()).collect();
    rows::table(
        directory,
        ("athletes.csv", schema::ATHLETES),
        athletes.len(),
        &["athlete_id"],
        |row| {
            let id = row.get("athlete_id")?;
            let athlete = athletes
                .get(id)
                .ok_or_else(|| rows::missing("athlete", id))?;
            identity(row, athlete, inputs)?;
            evidence(row, athlete)?;
            row.text(
                "performance_availability",
                if marked.contains(id) {
                    "eligible_selected_mark"
                } else {
                    "no_eligible_selected_mark_in_snapshot"
                },
            )
        },
    )
}

fn identity(row: &Row<'_>, athlete: &CanonicalAthlete, inputs: &Inputs<'_>) -> ReportResult<()> {
    if !inputs.dataset.schools.contains_key(&athlete.school) {
        return Err(rows::missing("school", athlete.school.as_str()));
    }
    row.text("display_name", &athlete.canonical_name)?;
    row.number("graduation_year", athlete.grad_year.get())?;
    row.text(
        "graduation_status",
        if athlete.derived_cohort_confidence() == Some(Confidence::HIGH) {
            "resolved_by_shared_cohort_policy"
        } else {
            "canonical_value_cohort_not_resolved"
        },
    )?;
    row.text("competition_category", athlete.gender.stable_key())?;
    row.text(
        "competition_category_status",
        if athlete.gender == Gender::Unknown {
            "unknown"
        } else {
            "observed"
        },
    )?;
    row.text("current_school_id", athlete.school.as_str())?;
    let status = inputs
        .dataset
        .identities()
        .status(athlete.id.as_str())
        .map_err(census_store::StoreError::from)?;
    row.text("identity_status", status.as_str())?;
    row.text("input_generation", &inputs.dataset.lineage.input_generation)?;
    row.literals(&[
        (
            "current_school_status",
            "canonical_school_currentness_unavailable",
        ),
        ("record_revision", "unavailable"),
        ("record_revision_status", "record_revision_not_retained"),
        ("affiliation_evidence_refs", "unavailable"),
        (
            "affiliation_evidence_status",
            "claim_level_affiliation_links_not_retained",
        ),
    ])
}

fn evidence(row: &Row<'_>, athlete: &CanonicalAthlete) -> ReportResult<()> {
    row.json(
        "source_identities",
        &(&athlete.source, &athlete.source_links),
    )?;
    row.json(
        "cohort_claims",
        &(&athlete.published_graduations, &athlete.observed_grades),
    )?;
    row.json(
        "entity_evidence_refs",
        &facts::references("athlete", athlete.id.as_str(), &athlete.evidence)?,
    )?;
    let published = athlete
        .published_graduations
        .iter()
        .map(|fact| rows::digest(&("published_graduation", athlete.id.as_str(), fact)));
    let grades = athlete
        .observed_grades
        .iter()
        .map(|fact| rows::digest(&("observed_grade", athlete.id.as_str(), fact)));
    let references: BTreeSet<_> = published.chain(grades).collect::<ReportResult<_>>()?;
    row.json("cohort_evidence_refs", &references)
}

pub(super) fn aliases(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let athletes: BTreeMap<_, _> = inputs
        .dataset
        .athletes
        .iter()
        .map(|athlete| (athlete.id.as_str(), athlete))
        .collect();
    let members: BTreeSet<_> = inputs
        .derivation
        .athletes()
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    let projection = inputs.dataset.identities();
    rows::table(
        directory,
        ("identity_aliases.csv", schema::ALIASES),
        athletes.len(),
        &["original_id"],
        |row| {
            row.text("input_generation", &inputs.dataset.lineage.input_generation)?;
            alias_row(row, &athletes, &members, &projection)
        },
    )
}

fn alias_row(
    row: &Row<'_>,
    athletes: &BTreeMap<&str, &CanonicalAthlete>,
    members: &BTreeSet<&str>,
    projection: &Arc<census_domain::model::AthleteIdentityProjection>,
) -> ReportResult<()> {
    let id = row.get("original_id")?;
    if !athletes.contains_key(id) {
        return Err(rows::missing("alias", id));
    }
    let canonical = projection.canonical_id(id);
    let included = members.contains(canonical);
    row.text(
        "canonical_id",
        if included { canonical } else { "unavailable" },
    )?;
    row.text("resolved_id", canonical)?;
    let status = projection
        .status(id)
        .map_err(census_store::StoreError::from)?;
    row.text("resolution_state", status.as_str())?;
    alias_transitions(row, id, projection)?;
    row.text(
        "publication_membership",
        if included {
            "canonical_in_publication"
        } else {
            "canonical_outside_publication"
        },
    )?;
    alias_literals(row)
}

fn alias_transitions(
    row: &Row<'_>,
    id: &str,
    projection: &Arc<census_domain::model::AthleteIdentityProjection>,
) -> ReportResult<()> {
    let references: BTreeSet<_> = projection
        .alias_transitions()
        .iter()
        .filter(|fact| fact.subject() == id)
        .map(|fact| fact.decision_id())
        .collect();
    row.json("resolution_reference", &references)?;
    row.text(
        "resolution_reference_status",
        if references.is_empty() {
            "no_retained_alias_transition"
        } else {
            "retained_projection_transitions"
        },
    )
}

fn alias_literals(row: &Row<'_>) -> ReportResult<()> {
    row.literals(&[
        ("entity_type", "athlete"),
        ("successor_ids", "unavailable"),
        ("effective_revision", "unavailable"),
        (
            "historical_state_status",
            "effective_revision_split_successors_supersession_not_retained",
        ),
    ])
}
