use crate::report::{ReportError, ReportResult};
use serde::Serialize;
use std::path::Path;

#[cfg(test)]
mod admission_tests;
mod index;
mod reference;
mod staging;
#[cfg(test)]
mod tests;

pub const REFERENCE_PROFILE: &str = "athletic-2026-us-boys-grade11-v1";
const REFERENCE_SHA256: &str = "8f613143f7e8e3fe8d8833314a85536297674b9bc6fb2bf7d253d1cc40bcda21";
const REFERENCE_BYTES: u64 = 10_147_760;
const REFERENCE_ROWS: usize = 142_705;

pub struct ComparisonRequest<'a> {
    pub workbook: &'a Path,
    pub reference: &'a Path,
    pub reference_profile: &'a str,
    pub graduation_year: i16,
}

#[derive(Debug, Serialize)]
pub struct ComparisonSummary {
    comparison_policy: &'static str,
    reference_profile: &'static str,
    reference_sha256: String,
    publication_generation: String,
    normalized_athletes_sha256: String,
    graduation_year: i16,
    traversal_complete: bool,
    required_identity_cohort_checks_passed: bool,
    full_reference_reproduction_verified: bool,
    counts: ComparisonCounts,
    supplemental_fields: SupplementalFields,
}

#[derive(Debug, Default, Serialize)]
pub struct ComparisonCounts {
    pub reference_rows: usize,
    pub matched_unique: usize,
    pub reference_only: usize,
    pub duplicate_reference_ids: usize,
    pub ambiguous_output_ids: usize,
    pub ambiguous_output_provider_ids_total: usize,
    pub many_reference_ids_to_one_output: usize,
    pub name_mismatches: usize,
    pub graduation_year_mismatches: usize,
    pub required_fields_unassessed: usize,
    pub output_only_provider_ids: usize,
    pub output_athletes_without_provider_id: usize,
    pub output_athletes: usize,
    pub output_athletes_without_unique_reference_match: usize,
}

#[derive(Debug, Serialize)]
struct SupplementalFields {
    teams_and_states: &'static str,
    events: &'static str,
    result_count: &'static str,
    other_observed_grades: &'static str,
    identity_match_status: &'static str,
    school_addresses: &'static str,
    personal_records: &'static str,
}

impl ComparisonSummary {
    pub fn passed(&self) -> bool {
        self.required_identity_cohort_checks_passed
    }
}

impl ComparisonCounts {
    fn passed(&self) -> bool {
        self.reference_only == 0
            && self.duplicate_reference_ids == 0
            && self.ambiguous_output_ids == 0
            && self.many_reference_ids_to_one_output == 0
            && self.name_mismatches == 0
            && self.graduation_year_mismatches == 0
            && self.required_fields_unassessed == 0
    }
}

pub fn compare(request: &ComparisonRequest<'_>) -> ReportResult<ComparisonSummary> {
    if request.reference_profile != REFERENCE_PROFILE || request.graduation_year != 2027 {
        return Err(error(
            "unsupported reference profile or graduation year; this profile requires 2027",
        ));
    }
    compare_bound(request, REFERENCE_BYTES, REFERENCE_SHA256, REFERENCE_ROWS)
}

fn compare_bound(
    request: &ComparisonRequest<'_>,
    reference_bytes: u64,
    reference_digest: &str,
    reference_rows: usize,
) -> ReportResult<ComparisonSummary> {
    let publication =
        super::publication::manifest::comparison_input(request.workbook, request.graduation_year)?;
    let (bytes, digest, rows) = publication.binding();
    staging::Staging::run(|stage| {
        let reference = stage.copy(request.reference, reference_bytes, reference_digest)?;
        let athletes = stage.copy(&publication.athletes(), bytes, digest)?;
        let mut index = index::Index::read(&athletes, bytes, rows)?;
        let mut counts = reference::compare(
            &reference,
            reference_rows,
            &mut index,
            request.graduation_year,
        )?;
        index.finish(&mut counts);
        Ok(ComparisonSummary {
            comparison_policy: "provider-id-name-graduation-v1",
            reference_profile: REFERENCE_PROFILE,
            reference_sha256: reference_digest.to_string(),
            publication_generation: publication.generation().to_string(),
            normalized_athletes_sha256: digest.to_string(),
            graduation_year: request.graduation_year,
            traversal_complete: true,
            required_identity_cohort_checks_passed: counts.passed(),
            full_reference_reproduction_verified: false,
            counts,
            supplemental_fields: SupplementalFields {
                teams_and_states: "not_assessed_temporal_affiliation_mapping_required",
                events: "not_assessed_reference_event_scope_mapping_required",
                result_count: "not_assessed_raw_results_are_not_best_mark_counts",
                other_observed_grades: "retained_reference_metadata_not_cohort_authority",
                identity_match_status: "historical_reference_status_not_identity_authority",
                school_addresses: "not_assessed_by_reference_publication_verification_required",
                personal_records: "not_assessed_by_reference_publication_verification_required",
            },
        })
    })
}

fn error(detail: impl Into<String>) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}
