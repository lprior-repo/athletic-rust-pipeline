use crate::kani::classify_kani_output;
use crate::kani::Outcome;

const SUCCESS_TAIL_1: &str = "SUMMARY:\n ** 0 of 122 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.05850434s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

const SUCCESS_TAIL_2: &str = "SUMMARY:\n ** 0 of 128 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.04390135s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

const SUCCESS_TAIL_3: &str = "SUMMARY:\n ** 0 of 59 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.03621107s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

const SUCCESS_TAIL_4: &str = "SUMMARY:\n ** 0 of 327 failed (4 unreachable)\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.12346949s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";

const FAIL_TAIL_1: &str = "CBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - kani_publish::check_professional_email_known_consumer\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

const FAIL_TAIL_2: &str = "CBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - kani_publish::check_professional_email_malformed\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

const FAIL_TAIL_3: &str = "CBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - store::kani::check_observation_id_bounds\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

const FAIL_TAIL_4: &str = "size of program expression: 659841 steps\nslicing removed 490160 assignments\nGenerated 47259 VCC(s), 9690 remaining after simplification\nRuntime Postprocess Equation: 0.497278s\nPassing problem to SMT2 QF_AUFBV using Z3\nCBMC failed\nVERIFICATION:- FAILED\nCBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.\n\nManual Harness Summary:\nVerification failed for - kani_id_mint::check_id_mint_golden_value\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.";

const TIMEOUT_TAIL: &str = "Timed out after 300 seconds";

const MISSING_TAIL: &str = "Error: no harness found";

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
fn classifier_rejects_mixed_success_and_failure() {
    let tail = "SUMMARY:\n ** 1 of 2 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.5s\nComplete - 1 successfully verified harnesses, 1 failures, 2 total.";
    assert_eq!(classify_kani_output("", tail), Outcome::Fail);
}

#[test]
fn classifier_rejects_verified_less_than_total() {
    let tail = "SUMMARY:\n ** 1 of 3 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.5s\nComplete - 2 successfully verified harnesses, 1 failures, 3 total.";
    assert_eq!(classify_kani_output("", tail), Outcome::Fail);
}

#[test]
fn classifier_rejects_uncounted_success() {
    assert_eq!(
        classify_kani_output("All checks were verified", ""),
        Outcome::BuildFail
    );
}

#[test]
fn classifier_rejects_success_with_build_error_stderr() {
    let stdout = "SUMMARY:\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.5s\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.";
    let stderr = "error: failed to build\n";
    assert_eq!(classify_kani_output(stdout, stderr), Outcome::BuildFail);
}

#[test]
fn classifier_rejects_wrong_total_count() {
    let tail = "SUMMARY:\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nVerification Time: 0.5s\nComplete - 1 successfully verified harnesses, 0 failures, 2 total.";
    assert_eq!(classify_kani_output("", tail), Outcome::Fail);
}

#[test]
fn classifier_rejects_non_digit_suffix() {
    let tail = "Complete - one successfully verified harnesses, zero failures, one total.";
    assert_eq!(classify_kani_output("", tail), Outcome::BuildFail);
}

#[test]
fn classifier_rejects_utf8_count() {
    let tail = "Complete - ① successfully verified harnesses, 0 failures, 1 total.";
    assert_eq!(classify_kani_output("", tail), Outcome::BuildFail);
}

#[test]
fn classifier_rejects_failure_before_a_successful_summary() {
    assert_eq!(
        classify_kani_output("VERIFICATION:- FAILED", SUCCESS_TAIL_1),
        Outcome::Fail
    );
}

#[test]
fn classifier_rejects_multiple_or_uncounted_complete_summaries() {
    for output in [
        "Complete - 1 successfully verified harnesses, 0 failures, 1 total.".to_string(),
        format!("{SUCCESS_TAIL_1}\n{SUCCESS_TAIL_2}"),
        "VERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 unrelated.".to_string(),
    ] {
        assert_eq!(classify_kani_output(&output, ""), Outcome::BuildFail);
    }
}

#[test]
fn classifier_rejects_success_for_more_than_the_single_selected_harness() {
    let output = "VERIFICATION:- SUCCESSFUL\nComplete - 2 successfully verified harnesses, 0 failures, 2 total.";
    assert_eq!(classify_kani_output("", output), Outcome::BuildFail);
}

#[test]
#[cfg(unix)]
fn a_nonzero_verifier_exit_cannot_certify_a_successful_summary() -> anyhow::Result<()> {
    let status = std::process::Command::new("/bin/false").status()?;
    check!(eq;
        super::classify_kani_process(status, "", SUCCESS_TAIL_1),
        Outcome::BuildFail
    );
    Ok(())
}
