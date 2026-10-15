use super::super::input::Inputs;
use super::{
    facts,
    rows::{self, Row},
    schema,
};
use crate::bests::SharedSelection;
use crate::export::public_identity::population::IdentityPopulation;
use crate::report::ReportResult;
use crate::workbook::verify::public_identity::{self, Expected};
use census_domain::model::{CanonicalAthlete, Confidence, Gender};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

pub(super) mod public;
use public::{alias_headers, alias_public_identity, public_identity_row};

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    bests: &[SharedSelection],
) -> ReportResult<u64> {
    let athletes: BTreeMap<_, _> = inputs
        .derivation
        .athletes()
        .iter()
        .map(|row| {
            Ok((
                Expected::of(inputs.dataset, row.id.as_str())?.athlete_id,
                row,
            ))
        })
        .collect::<ReportResult<_>>()?;
    if athletes.len() != inputs.derivation.athletes().len() {
        return Err(super::super::defect(
            "selected athlete inventory repeats a public identity".into(),
        ));
    }
    let marked: BTreeSet<_> = bests.iter().map(|row| row.athlete_id().as_str()).collect();
    rows::table(
        directory,
        (
            "athletes.csv",
            &public_identity::headers(inputs.dataset, schema::ATHLETES),
        ),
        athletes.len(),
        &["athlete_id"],
        |row| {
            let id = row.get("athlete_id")?;
            let athlete = athletes
                .get(id)
                .ok_or_else(|| rows::missing("athlete", id))?;
            public_identity_row(row, inputs.dataset, athlete.id.as_str())?;
            identity(row, athlete, inputs)?;
            evidence(row, athlete)?;
            row.text(
                "performance_availability",
                if marked.contains(athlete.id.as_str()) {
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
    let status = if inputs.dataset.selected_profile_id().is_some() {
        Expected::of(inputs.dataset, athlete.id.as_str())?;
        "accepted_public_identity"
    } else {
        inputs
            .dataset
            .identities()
            .status(athlete.id.as_str())
            .map_err(census_store::StoreError::from)?
            .as_str()
    };
    row.text("identity_status", status)?;
    row.text("input_generation", &inputs.dataset.lineage.input_generation)?;
    affiliation(row, athlete, inputs)?;
    row.literals(&[
        ("record_revision", "unavailable"),
        ("record_revision_status", "record_revision_not_retained"),
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

pub(super) fn aliases(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<u64> {
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
    let headers: Vec<_> = alias_headers(inputs.dataset)
        .into_iter()
        .map(
            |header| match (inputs.dataset.selected_profile_id(), header) {
                (Some(_), "resolution_state") => "historical_automatic_identity_status",
                _ => header,
            },
        )
        .collect();
    rows::table(
        directory,
        ("identity_aliases.csv", &headers),
        athletes.len(),
        &["original_id"],
        |row| {
            row.text("input_generation", &inputs.dataset.lineage.input_generation)?;
            alias_public_identity(row, inputs.dataset)?;
            alias_row(row, &athletes, &members, inputs)
        },
    )
}

fn alias_row(
    row: &Row<'_>,
    athletes: &BTreeMap<&str, &CanonicalAthlete>,
    members: &BTreeSet<&str>,
    inputs: &Inputs<'_>,
) -> ReportResult<()> {
    let id = row.get("original_id")?;
    if !athletes.contains_key(id) {
        return Err(rows::missing("alias", id));
    }
    let identities = IdentityPopulation::of(inputs.dataset)?;
    let canonical = identities
        .canonical_candidate(id)
        .ok_or_else(|| rows::missing("publication identity candidate", id))?;
    let included = members.contains(canonical);
    row.text(
        "canonical_id",
        if included { canonical } else { "unavailable" },
    )?;
    row.text("resolved_id", canonical)?;
    let projection = inputs.dataset.identities();
    let status = projection
        .status(id)
        .map_err(census_store::StoreError::from)?;
    let status_header = if inputs.dataset.selected_profile_id().is_some() {
        "historical_automatic_identity_status"
    } else {
        "resolution_state"
    };
    row.text(status_header, status.as_str())?;
    alias_transitions(row, id, &projection)?;
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

fn affiliation(row: &Row<'_>, athlete: &CanonicalAthlete, inputs: &Inputs<'_>) -> ReportResult<()> {
    match inputs.derivation.current_affiliation(&athlete.id) {
        None => row.literals(&[
            (
                "current_school_status",
                "canonical_school_currentness_unavailable",
            ),
            ("affiliation_evidence_refs", "unavailable"),
            (
                "affiliation_evidence_status",
                "claim_level_affiliation_links_not_retained",
            ),
        ]),
        Some(resolution) => {
            row.text("current_school_status", resolution.status())?;
            row.json("affiliation_evidence_refs", &resolution.evidence_refs())?;
            row.text(
                "affiliation_evidence_status",
                if resolution.evidence_refs().is_empty() {
                    "no_retained_affiliation_evidence"
                } else {
                    "retained_selected_affiliation_evidence"
                },
            )
        }
    }
}

#[cfg(test)]
mod tests;
