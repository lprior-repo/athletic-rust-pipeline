/// Focused tests for cohort classification.
use crate::alpha_cohort::{classify_cohort, CohortDecision};

#[test]
fn explicit_year_match_includes() {
    let result = classify_cohort(2027, Some(2027), None, None);
    assert!(matches!(result, CohortDecision::Include(_)));
    let msg = result.message();
    assert!(msg.contains("2027"));
}

#[test]
fn explicit_year_mismatch_excludes() {
    let result = classify_cohort(2027, Some(2028), None, None);
    assert!(matches!(result, CohortDecision::Exception(_)));
}

#[test]
fn explicit_year_conflict_with_grade_is_exception() {
    let result = classify_cohort(2027, Some(2026), None, Some(11));
    assert!(matches!(result, CohortDecision::Exception(_)));
    let msg = result.message();
    assert!(msg.contains("conflict"));
}

#[test]
fn grade_11_2025_26_fallback_includes() {
    let result = classify_cohort(2027, None, Some("2025-26"), Some(11));
    assert!(matches!(result, CohortDecision::Include(_)));
}

#[test]
fn grade_12_2026_27_fallback_includes() {
    let result = classify_cohort(2027, None, Some("2026-27"), Some(12));
    assert!(matches!(result, CohortDecision::Include(_)));
}

#[test]
fn missing_evidence_is_exception() {
    let result = classify_cohort(2027, None, None, None);
    assert!(matches!(result, CohortDecision::Exception(_)));
}

#[test]
fn season_label_trimmed_before_comparison() {
    let result = classify_cohort(2027, None, Some("  2025-26  "), Some(11));
    assert!(matches!(result, CohortDecision::Include(_)));
}

#[test]
fn grade_12_in_wrong_season_is_exception() {
    let result = classify_cohort(2027, None, Some("2025-26"), Some(12));
    assert!(matches!(result, CohortDecision::Exception(_)));
}

#[test]
fn grade_11_in_wrong_season_is_exception() {
    let result = classify_cohort(2027, None, Some("2026-27"), Some(11));
    assert!(matches!(result, CohortDecision::Exception(_)));
}

#[test]
fn explicit_year_2027_with_conflicting_grade_still_exception() {
    let result = classify_cohort(2027, Some(2026), Some("2025-26"), Some(11));
    assert!(matches!(result, CohortDecision::Exception(_)));
    let msg = result.message();
    assert!(msg.contains("conflict"));
}

#[test]
fn explicit_year_2027_without_grade_still_exception_for_wrong_year() {
    let result = classify_cohort(2027, Some(2028), None, None);
    assert!(matches!(result, CohortDecision::Exception(_)));
}

#[test]
fn cohort_decision_message_returns_inner_string() {
    let inc = CohortDecision::Include("test".to_owned());
    let exc = CohortDecision::Exclude("test".to_owned());
    let exn = CohortDecision::Exception("test".to_owned());
    assert_eq!(inc.message(), "test");
    assert_eq!(exc.message(), "test");
    assert_eq!(exn.message(), "test");
}
