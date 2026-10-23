use std::path::Path;

use crate::report::ReportResult;

use super::input::Inputs;

mod athletes;
mod best_marks;
mod classes;
mod combined_exclusions;
mod contacts;
mod coverage;
mod facts;
mod inventory;
mod reported_performances;
mod revision;
pub(in crate::workbook::publication) mod rows;
mod schema;
mod schools;
mod source_profile_prs;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    artifact_revision: u32,
) -> ReportResult<()> {
    let outcome = inputs.outcome()?;
    verify_outcome(directory, inputs, artifact_revision, &outcome).map(|_| ())
}

pub(super) fn verify_outcome(
    directory: &Path,
    inputs: &Inputs<'_>,
    artifact_revision: u32,
    outcome: &crate::bests::SelectionOutcome,
) -> ReportResult<super::super::inventory::RowCounts> {
    super::super::versions::validate_policy(
        artifact_revision,
        inputs.dataset.lineage.policy_revision,
    )?;
    super::super::selected::binding(artifact_revision, inputs.dataset)?;
    inputs.derivation.ensure_selected_affiliation()?;
    let revision = revision::Revision::parse(artifact_revision)?;
    inventory::verify(inputs)?;
    inputs.admission.ensure_dataset(inputs.dataset)?;
    let bests = &outcome.selections;
    let schools = schools::inventory(inputs, bests)?;
    let coaches = crate::workbook::recruiting::bounded::bounded_claims(
        inputs.derivation.coach_observations(),
        crate::workbook::recruiting::bounded::WORKBOOK_GROUP_BUDGET,
    )?;
    let facts = facts::inventory(inputs, &schools, &coaches, bests, revision)?;
    let mut counts = core_counts(
        directory, inputs, bests, &schools, &coaches, &facts, revision,
    )?;
    insert_exclusion_counts(directory, inputs, outcome, revision, &mut counts)?;
    if matches!(
        artifact_revision,
        crate::workbook::publication::selected::REPORTED_REVISION
            | crate::workbook::publication::selected::PROFILE_PRS_REVISION
            | crate::workbook::publication::selected::OWNED_PROFILE_PRS_REVISION
    ) {
        let projection = crate::workbook::reported::Projection::of(
            inputs.dataset,
            &inputs.derivation,
            &inputs.admission,
            bests,
        )?
        .ok_or_else(|| {
            super::defect("reported revision lacks Include reported projection".into())
        })?;
        counts.insert(
            "reported_performances.csv".into(),
            reported_performances::verify(directory, &projection)?,
        );
    }
    if matches!(
        artifact_revision,
        crate::workbook::publication::selected::PROFILE_PRS_REVISION
            | crate::workbook::publication::selected::OWNED_PROFILE_PRS_REVISION
    ) {
        counts.insert(
            "source_profile_prs.csv".into(),
            source_profile_prs::verify(directory, inputs, outcome)?,
        );
    }
    Ok(counts)
}

fn core_counts(
    directory: &Path,
    inputs: &Inputs<'_>,
    bests: &[crate::bests::SharedSelection],
    schools: &schools::Schools<'_>,
    coaches: &[census_domain::model::CanonicalCoach],
    facts: &facts::Facts<'_>,
    revision: revision::Revision,
) -> ReportResult<super::super::inventory::RowCounts> {
    Ok(std::collections::BTreeMap::from([
        (
            "athletes.csv".into(),
            athletes::verify(directory, inputs, bests)?,
        ),
        (
            "schools.csv".into(),
            schools::verify(directory, inputs, schools)?,
        ),
        (
            "comparison_classes.csv".into(),
            classes::verify(directory, inputs, bests)?,
        ),
        (
            "best_marks.csv".into(),
            best_marks::verify(directory, inputs, bests, revision)?,
        ),
        (
            "coach_contacts.csv".into(),
            contacts::verify(directory, inputs, coaches, schools)?,
        ),
        (
            "provenance.csv".into(),
            facts::verify(directory, inputs, facts)?,
        ),
        ("coverage.csv".into(), coverage::verify(directory, inputs)?),
        (
            "identity_aliases.csv".into(),
            athletes::aliases(directory, inputs)?,
        ),
    ]))
}

fn insert_exclusion_counts(
    directory: &Path,
    inputs: &Inputs<'_>,
    outcome: &crate::bests::SelectionOutcome,
    revision: revision::Revision,
    counts: &mut super::super::inventory::RowCounts,
) -> ReportResult<()> {
    if revision.has_exclusions() {
        counts.insert(
            "combined_exclusions.csv".into(),
            combined_exclusions::verify(directory, inputs, &outcome.combined_exclusions)?,
        );
    }
    if revision.has_participation_exclusions() {
        counts.insert(
            "current_affiliations.csv".into(),
            super::super::current_affiliations::verify(directory, inputs.dataset)?,
        );
        counts.insert(
            "participation_exclusions.csv".into(),
            super::super::participation_exclusions::verify(
                directory,
                inputs.dataset,
                inputs.dataset.verified_participation()?,
                outcome,
            )?,
        );
    }
    Ok(())
}

pub(in crate::workbook) use rows::Row as NormalizedRow;

pub(in crate::workbook) fn visit_athletes(
    path: &Path,
    expected: usize,
    artifact_revision: u32,
    visit: impl FnMut(&NormalizedRow<'_>) -> ReportResult<()>,
) -> ReportResult<u64> {
    let additional = match artifact_revision {
        1..=6 => None,
        7 | 8 | 10..=14 => Some(["canonical_candidate_id", "identity_resolution"]),
        _ => {
            return Err(super::defect(format!(
                "unsupported comparison artifact revision {artifact_revision}"
            )))
        }
    };
    let headers: Vec<_> = schema::ATHLETES
        .iter()
        .copied()
        .chain(additional.into_iter().flatten())
        .collect();
    rows::read_table(path, &headers, expected, visit)
}
