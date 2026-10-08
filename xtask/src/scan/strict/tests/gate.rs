use super::*;

#[test]
fn missing_invalid_and_zero_coverage_measurements_fail_closed() -> TestResult {
    for invalid in [
        serde_json::json!({}),
        report(Findings::default()),
        serde_json::json!({
            "structure": { "strict_policy": "lexical-25-lines-5-parameters-v1", "strict_files_checked": 1,
                "functions_over_25_logical_lines": 0, "functions_over_25_sites": ["violation"] }
        }),
    ] {
        check!(validate_report(&invalid).is_err());
    }
    Ok(())
}

#[test]
fn forged_clean_summary_cannot_certify_current_production() -> TestResult {
    let forged = report(inspect("fn fake() {}")?);
    check!(enforce_report(&forged).is_err());
    Ok(())
}

#[test]
fn truncated_mismatched_and_nonstring_sites_fail_before_authentication() -> TestResult {
    let complete = report(inspect("fn run() {}")?);
    for key in STRICT_FIELDS {
        let mut truncated = complete.clone();
        truncated
            .get_mut("structure")
            .and_then(Value::as_object_mut)
            .context("fixture structure missing")?
            .remove(*key);
        check!(validate_report(&truncated).is_err());
    }
    for sites in [
        serde_json::json!([null]),
        serde_json::json!([""]),
        serde_json::json!(["hidden violation"]),
    ] {
        let mut invalid = complete.clone();
        invalid
            .get_mut("structure")
            .and_then(Value::as_object_mut)
            .context("fixture structure missing")?
            .insert("functions_over_25_sites".to_string(), sites);
        check!(validate_report(&invalid).is_err());
    }
    Ok(())
}

#[test]
fn allow_increase_cannot_write_baseline_for_a_strict_violation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let baseline = directory.path().join("baseline.json");
    let clippy = directory.path().join("clippy.tsv");
    let scan = directory.path().join("scan.json");
    std::fs::write(&baseline, "{\"sentinel\":\"preserve\"}")?;
    std::fs::write(&clippy, "")?;
    std::fs::write(&scan, serde_json::to_vec(&report(inspect(&function(61))?))?)?;
    check!(crate::baseline::update(&baseline, &clippy, &scan, true).is_err());
    check!(eq; std::fs::read_to_string(&baseline)?, "{\"sentinel\":\"preserve\"}");
    directory.close()?;
    Ok(())
}

#[test]
fn ratchet_rejects_existing_strict_debt_without_growth() -> TestResult {
    let directory = tempfile::tempdir()?;
    let baseline = directory.path().join("baseline.json");
    let clippy = directory.path().join("clippy.tsv");
    let scan = directory.path().join("scan.json");
    let measured = report(inspect(&function(61))?);
    let structure = measured
        .get("structure")
        .context("fixture structure missing")?;
    std::fs::write(
        &baseline,
        serde_json::to_vec(&serde_json::json!({
            "clippy": {}, "scan": {}, "structure": structure,
        }))?,
    )?;
    std::fs::write(&clippy, "")?;
    std::fs::write(&scan, serde_json::to_vec(&measured)?)?;
    check!(crate::baseline::ratchet(&baseline, &clippy, &scan).is_err());
    directory.close()?;
    Ok(())
}
