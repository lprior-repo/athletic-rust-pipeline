use super::{encode, provenance, text, write_csv};
use crate::bests::SharedSelection;
use crate::report::{Derivation, ReportResult};
use crate::workbook::public_identity::{self, Identity};
use census_domain::model::{AthleteId, CanonicalAthlete, Gender};
use std::collections::BTreeSet;
use std::path::Path;

const HEADER: [&str; 19] = [
    "athlete_id",
    "display_name",
    "graduation_year",
    "graduation_status",
    "competition_category",
    "competition_category_status",
    "current_school_id",
    "current_school_status",
    "cohort_evidence_refs",
    "affiliation_evidence_refs",
    "performance_availability",
    "source_identities",
    "record_revision",
    "record_revision_status",
    "input_generation",
    "cohort_claims",
    "identity_status",
    "entity_evidence_refs",
    "affiliation_evidence_status",
];

pub(super) fn write(
    directory: &Path,
    derivation: &Derivation<'_>,
    bests: &[SharedSelection],
) -> ReportResult<u64> {
    public_identity::preflight(derivation.dataset(), derivation.athletes())?;
    let view = public_identity::view(derivation.dataset())?;
    let header = public_identity::headers(&HEADER, view);
    let marked: BTreeSet<_> = bests.iter().map(SharedSelection::athlete_id).collect();
    write_csv(&directory.join("athletes.csv"), &header, |writer| {
        derivation
            .athletes()
            .iter()
            .try_for_each(|athlete| writer.push_vec(row(athlete, derivation, &marked)?))
    })
}

fn row(
    athlete: &CanonicalAthlete,
    derivation: &Derivation<'_>,
    marked: &BTreeSet<&AthleteId>,
) -> ReportResult<Vec<String>> {
    let dataset = derivation.dataset();
    let identity = Identity::of(public_identity::view(dataset)?, athlete.id.as_str())?;
    let status = match &identity {
        Identity::Selected(_) => "accepted_public_identity",
        Identity::Historical(_) => dataset
            .identities()
            .status(athlete.id.as_str())
            .map_err(census_store::StoreError::from)?
            .as_str(),
    };
    let evidence = provenance::references("athlete", athlete.id.as_str(), &athlete.evidence)?;
    let has_mark = marked.contains(&athlete.id);
    let [school_status, affiliation_refs, affiliation_status] = affiliation(athlete, derivation)?;
    let fields = [
        identity.athlete_id().to_owned(),
        athlete.canonical_name.clone(),
        athlete.grad_year.get().to_string(),
        if athlete.derived_cohort_confidence() == Some(census_domain::model::Confidence::HIGH) {
            "resolved_by_shared_cohort_policy"
        } else {
            "canonical_value_cohort_not_resolved"
        }
        .into(),
        athlete.gender.stable_key().into(),
        if athlete.gender == Gender::Unknown {
            "unknown"
        } else {
            "observed"
        }
        .into(),
        text(Some(athlete.school.as_str())),
        school_status,
        provenance::cohort_references(athlete)?,
        affiliation_refs,
        if has_mark {
            "eligible_selected_mark"
        } else {
            "no_eligible_selected_mark_in_snapshot"
        }
        .into(),
        encode(&(&athlete.source, &athlete.source_links))?,
        "unavailable".into(),
        "record_revision_not_retained".into(),
        dataset.lineage.input_generation.clone(),
        encode(&(&athlete.published_graduations, &athlete.observed_grades))?,
        status.into(),
        evidence,
        affiliation_status,
    ];
    Ok(fields.into_iter().chain(identity.audit()?).collect())
}

fn affiliation(
    athlete: &CanonicalAthlete,
    derivation: &Derivation<'_>,
) -> ReportResult<[String; 3]> {
    match derivation.current_affiliation(&athlete.id) {
        None => Ok([
            "canonical_school_currentness_unavailable".into(),
            "unavailable".into(),
            "claim_level_affiliation_links_not_retained".into(),
        ]),
        Some(resolution) => Ok([
            resolution.status().into(),
            encode(&resolution.evidence_refs())?,
            if resolution.evidence_refs().is_empty() {
                "no_retained_affiliation_evidence"
            } else {
                "retained_selected_affiliation_evidence"
            }
            .into(),
        ]),
    }
}

#[cfg(test)]
mod tests;
