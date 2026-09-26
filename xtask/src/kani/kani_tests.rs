//! Tests for the Kani output classifier.
//!
//! Fixture tails are from `docs/VERIFICATION-EVIDENCE.md` (real cargo-kani 0.67.0 output).

use crate::kani::classify_kani_output;
use crate::kani::Outcome;

/// T1 — `check_gradyear_of_formula` (verified)
const SUCCESS_TAIL_1: &str = "SUMMARY:\n ** 0 of 122 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.05850434s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

/// T2 — `check_gradyear_of_known_values` (verified)
const SUCCESS_TAIL_2: &str = "SUMMARY:\n ** 0 of 128 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.04390135s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

/// T3 — `check_gradyear_of_saturating` (verified)
const SUCCESS_TAIL_3: &str = "SUMMARY:\n ** 0 of 59 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.03621107s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

/// T4 — `check_observed_grade_grad_year` (verified)
const SUCCESS_TAIL_4: &str = "SUMMARY:\n ** 0 of 327 failed (4 unreachable)\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.12346949s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

/// T5 — `check_professional_email_known_consumer` (CBMC OOM → FAILED)
const FAIL_TAIL_1: &str = "CBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - kani_publish::check_professional_email_known_consumer\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

/// T6 — `check_professional_email_malformed` (CBMC OOM → FAILED)
const FAIL_TAIL_2: &str = "CBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - kani_publish::check_professional_email_malformed\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

/// T8 — `check_observation_id_bounds` (CBMC OOM → FAILED)
const FAIL_TAIL_3: &str = "CBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - store::kani::check_observation_id_bounds\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

/// T9 — `check_id_mint_golden_value` (solver timeout → FAILED)
const FAIL_TAIL_4: &str = "size of program expression: 659841 steps\nslicing removed 490160 assignments\nGenerated 47259 VCC(s), 9690 remaining after simplification\nRuntime Postprocess Equation: 0.497278s\nPassing problem to SMT2 QF_AUFBV using Z3\nCBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - kani_id_mint::check_id_mint_golden_value\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

const TIMEOUT_TAIL: &str = "Timed out after 300 seconds";

const MISSING_TAIL: &str = "Error: no harness found";

const LEGACY_SUCCESS: &str = "All checks were verified";

#[test]
fn classifier_accepts_success_t1() {
    assert_eq!(classify_kani_output("", SUCCESS_TAIL_1), Outcome::Pass);
}

#[test]
fn classifier_accepts_success_t2() {
    assert_eq!(classify_kani_output("", SUCCESS_TAIL_2), Outcome::Pass);
}

#[test]
fn classifier_accepts_success_t3() {
    assert_eq!(classify_kani_output("", SUCCESS_TAIL_3), Outcome::Pass);
}

#[test]
fn classifier_accepts_success_t4() {
    assert_eq!(classify_kani_output("", SUCCESS_TAIL_4), Outcome::Pass);
}

#[test]
fn classifier_rejects_fail_t5() {
    assert_eq!(classify_kani_output("", FAIL_TAIL_1), Outcome::Fail);
}

#[test]
fn classifier_rejects_fail_t6() {
    assert_eq!(classify_kani_output("", FAIL_TAIL_2), Outcome::Fail);
}

#[test]
fn classifier_rejects_fail_t8() {
    assert_eq!(classify_kani_output("", FAIL_TAIL_3), Outcome::Fail);
}

#[test]
fn classifier_rejects_fail_t9() {
    assert_eq!(classify_kani_output("", FAIL_TAIL_4), Outcome::Fail);
}

#[test]
fn classifier_accepts_timeout() {
    assert_eq!(classify_kani_output("", TIMEOUT_TAIL), Outcome::Timeout);
}

#[test]
fn classifier_rejects_missing_harness() {
    assert_eq!(classify_kani_output("", MISSING_TAIL), Outcome::Missing);
}

#[test]
fn classifier_accepts_legacy_success() {
    assert_eq!(classify_kani_output(LEGACY_SUCCESS, ""), Outcome::Pass);
}

#[test]
fn classifier_rejects_zero_verified() {
    let tail = "SUMMARY:\n ** 0 of 122 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.05850434s\nComplete - 0 successfully verified harnesses, 0 failures, 1 total.";
    assert_eq!(classify_kani_output("", tail), Outcome::BuildFail);
}

#[test]
fn classifier_accepts_success_with_stdout_header() {
    let stdout = "   Compiling census-domain v0.1.0\n";
    let stderr = "SUMMARY:\n ** 0 of 122 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.05850434s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";
    assert_eq!(classify_kani_output(stdout, stderr), Outcome::Pass);
}

#[test]
fn classifier_accepts_success_via_stdout_only() {
    let stdout = "SUMMARY:\n ** 0 of 122 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.05850434s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";
    assert_eq!(classify_kani_output(stdout, ""), Outcome::Pass);
}

#[test]
fn classifier_rejects_unknown_output() {
    assert_eq!(classify_kani_output("", ""), Outcome::BuildFail);
}

#[test]
fn classifier_rejects_cbmc_failed_without_verdict() {
    let tail = "CBMC failed\nCBMC appears to have run out of memory.\n";
    assert_eq!(classify_kani_output("", tail), Outcome::BuildFail);
}

#[test]
fn mandatory_names_not_in_known_harness() {
    use crate::kani::KNOWN_HARNESS;

    let mandatory = &[
        "check_fixed_point_bounds",
        "check_pr_comparison_laws",
        "check_identity_contradiction",
        "check_redirect_cycle",
        "check_retry_limit",
        "check_terminal_state_no_retry",
        "check_store_batch_arithmetic",
        "check_census_scope",
    ];

    let by_name: std::collections::HashMap<&str, &crate::kani::HarnessInfo> =
        KNOWN_HARNESS.iter().map(|h| (h.name, h)).collect();

    let mut missing = Vec::new();
    for &name in mandatory {
        if !by_name.contains_key(name) {
            missing.push(name);
        }
    }
    assert_eq!(
        missing.len(),
        8,
        "all 8 contract names must be absent (pre-execution failure expected)"
    );
}
