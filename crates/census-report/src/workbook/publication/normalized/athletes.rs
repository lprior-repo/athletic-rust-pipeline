use super::{encode, provenance, text, write_csv};
use crate::bests::SharedSelection;
use crate::report::{Derivation, ReportResult};
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
    let marked: BTreeSet<_> = bests.iter().map(SharedSelection::athlete_id).collect();
    write_csv(&directory.join("athletes.csv"), &HEADER, |writer| {
        for athlete in derivation.athletes() {
            writer.push(row(athlete, derivation, &marked)?)?;
        }
        Ok(())
    })
}

fn row(
    athlete: &CanonicalAthlete,
    derivation: &Derivation<'_>,
    marked: &BTreeSet<&AthleteId>,
) -> ReportResult<[String; 19]> {
    let dataset = derivation.dataset();
    let status = dataset
        .identities()
        .status(athlete.id.as_str())
        .map_err(census_store::StoreError::from)?;
    let evidence = provenance::references("athlete", athlete.id.as_str(), &athlete.evidence)?;
    let has_mark = marked.contains(&athlete.id);
    Ok([
        athlete.id.to_string(),
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
        "canonical_school_currentness_unavailable".into(),
        provenance::cohort_references(athlete)?,
        "unavailable".into(),
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
        status.as_str().into(),
        evidence,
        "claim_level_affiliation_links_not_retained".into(),
    ])
}
