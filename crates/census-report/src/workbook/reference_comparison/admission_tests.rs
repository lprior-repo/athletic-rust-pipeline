use super::{compare_bound, staging::Staging, ComparisonRequest, REFERENCE_PROFILE};
use crate::workbook::publication::manifest::{
    comparison_input,
    tests::{options, seed},
};
use census_domain::model::{SourceIdentity, SourceNamespace};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn publication(root: &Path, limit: Option<usize>) -> TestResult<PathBuf> {
    let store = Store::open(root.join("store"))?;
    seed(&store, "Synthetic Publishing High")?;
    let mut athlete = store
        .snapshot()
        .athletes()?
        .into_iter()
        .next()
        .ok_or("fixture athlete")?;
    athlete.source_links.push(SourceIdentity::new(
        SourceNamespace::athletic_net("athlete"),
        "1",
    ));
    store.append(Table::Athletes, &athlete)?;
    let mut options = options()?;
    options.limit = limit;
    Ok(crate::workbook::build(&store, &options)?)
}

fn request<'a>(workbook: &'a Path, reference: &'a Path) -> ComparisonRequest<'a> {
    ComparisonRequest {
        workbook,
        reference,
        reference_profile: REFERENCE_PROFILE,
        graduation_year: 2027,
    }
}

#[test]
fn manifested_generation_admits_exact_summary_and_reports_discrepancy() -> TestResult {
    let root = tempfile::tempdir()?;
    let workbook = publication(root.path(), None)?;
    let reference = root.path().join("reference.xlsx");
    for (name, passed, mismatches) in [("Ada Runner", true, 0), ("Different Runner", false, 1)] {
        super::tests::reference_book(&reference, "All Athletes", "AthleteID", name)?;
        let bytes = std::fs::read(&reference)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let summary = compare_bound(
            &request(&workbook, &reference),
            u64::try_from(bytes.len())?,
            &digest,
            1,
        )?;
        check!(eq; summary.passed(), passed);
        let output = serde_json::to_value(&summary)?;
        check!(eq; output["traversal_complete"], serde_json::json!(true));
        check!(eq; output["required_identity_cohort_checks_passed"], serde_json::json!(passed));
        check!(eq; output["full_reference_reproduction_verified"], serde_json::json!(false));
        check!(output.get("complete").is_none());
        check!(output.get("passed").is_none());
        check!(eq; output["reference_sha256"], serde_json::json!(digest));
        check!(eq; output["counts"]["matched_unique"], serde_json::json!(1));
        check!(eq; output["counts"]["name_mismatches"], serde_json::json!(mismatches));
        check!(eq; output["counts"]["required_fields_unassessed"], serde_json::json!(0));
        let admitted = comparison_input(&workbook, 2027)?;
        check!(eq; output["publication_generation"], serde_json::json!(admitted.generation()));
        check!(eq; output["normalized_athletes_sha256"], serde_json::json!(admitted.binding().1));
        let encoded = serde_json::to_string(&summary)?;
        for secret in [
            "Ada Runner",
            "Different Runner",
            "Synthetic Publishing High",
            "source_identities",
            "athlete_id",
        ] {
            check!(!encoded.contains(secret));
        }
        let keys = output
            .as_object()
            .ok_or("summary object")?
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        check!(eq;
            keys,
            std::collections::BTreeSet::from([
                "comparison_policy",
                "reference_profile",
                "reference_sha256",
                "publication_generation",
                "normalized_athletes_sha256",
                "graduation_year",
                "traversal_complete",
                "required_identity_cohort_checks_passed",
                "full_reference_reproduction_verified",
                "counts",
                "supplemental_fields",
            ])
        );
        check!(eq;
            output["supplemental_fields"],
            serde_json::json!({
                "teams_and_states": "not_assessed_temporal_affiliation_mapping_required",
                "events": "not_assessed_reference_event_scope_mapping_required",
                "result_count": "not_assessed_raw_results_are_not_best_mark_counts",
                "other_observed_grades": "retained_reference_metadata_not_cohort_authority",
                "identity_match_status": "historical_reference_status_not_identity_authority",
                "school_addresses": "not_assessed_by_reference_publication_verification_required",
                "personal_records": "not_assessed_by_reference_publication_verification_required",
            })
        );
    }
    check!(super::compare(&request(&workbook, &reference)).is_err());
    Ok(())
}

#[test]
fn manifested_generation_requires_unlimited_matching_year() -> TestResult {
    let root = tempfile::tempdir()?;
    let workbook = publication(root.path(), None)?;
    check!(comparison_input(&workbook, 2027).is_ok());
    let error = comparison_input(&workbook, 2028)
        .err()
        .ok_or("wrong year admitted")?;
    check!(error
        .to_string()
        .contains("unlimited matching graduation-year"));
    let limited = tempfile::tempdir()?;
    let workbook = publication(limited.path(), Some(1))?;
    let error = comparison_input(&workbook, 2027)
        .err()
        .ok_or("limited generation admitted")?;
    check!(error
        .to_string()
        .contains("unlimited matching graduation-year"));
    Ok(())
}

#[test]
fn admitted_csv_binding_refuses_same_length_tamper_and_retains_staged_bytes() -> TestResult {
    let root = tempfile::tempdir()?;
    let workbook = publication(root.path(), None)?;
    let admitted = comparison_input(&workbook, 2027)?;
    let source = admitted.athletes();
    let original = std::fs::read(&source)?;
    let mut altered = original.clone();
    let first = altered.first_mut().ok_or("empty CSV")?;
    *first = if *first == b'x' { b'y' } else { b'x' };
    let (bytes, digest, _) = admitted.binding();
    Staging::run(|stage| {
        let staged = stage.copy(&source, bytes, digest)?;
        std::fs::write(&source, &altered)
            .map_err(|cause| crate::report::io_error(&source, cause))?;
        let retained =
            std::fs::read(staged).map_err(|cause| crate::report::io_error(&source, cause))?;
        if retained != original {
            return Err(super::error("staged CSV changed with its source"));
        }
        if stage.copy(&source, bytes, digest).is_ok() {
            return Err(super::error("same-length CSV tampering was accepted"));
        }
        Ok(())
    })?;
    check!(comparison_input(&workbook, 2027).is_err());
    Ok(())
}
