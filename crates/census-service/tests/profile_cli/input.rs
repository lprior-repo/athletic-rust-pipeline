use census_store::MAX_PROFILE_RECORD_BYTES;

use super::fixture::{equal, profile, TestResult};
use super::harness::{invoke, path_text, refusal};

#[test]
fn profile_cli_no_store_does_not_open_the_default_root_or_read_input() -> TestResult {
    let directory = tempfile::tempdir()?;
    let result = invoke(
        directory.path(),
        None,
        &["profile", "register", "--file", "missing.json"],
    )?;
    refusal(result, "explicit_store_required")?;
    equal(directory.path().join("var").exists(), false)
}

#[test]
fn profile_cli_invalid_input_is_refused_before_the_explicit_store_is_opened() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("never-opened");
    let file = directory.path().join("profile.json");
    let too_large = MAX_PROFILE_RECORD_BYTES
        .checked_add(1)
        .ok_or("fixture size overflow")?;
    for (bytes, code) in [
        (br#"{"version":2}"#.to_vec(), "input_version_unsupported"),
        (b"not JSON".to_vec(), "input_json_invalid"),
        (vec![b' '; too_large], "input_oversized"),
    ] {
        std::fs::write(&file, bytes)?;
        refusal(
            invoke(
                directory.path(),
                Some(&root),
                &["profile", "register", "--file", path_text(&file)?],
            )?,
            code,
        )?;
        equal(root.exists(), false)?;
    }
    Ok(())
}

#[test]
fn profile_cli_invalid_selection_is_refused_before_opening_its_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("never-opened");
    let file = directory.path().join("selection.json");
    for bytes in [
        br#"{"version":2}"#.as_slice(),
        br#"{"version":1,"store_identity":"bad","profile_id":"bad"}"#.as_slice(),
    ] {
        std::fs::write(&file, bytes)?;
        let output = invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--selection", path_text(&file)?],
        )?;
        let expected = if bytes == br#"{"version":2}"# {
            "input_version_unsupported"
        } else {
            "input_json_invalid"
        };
        refusal(output, expected)?;
        equal(root.exists(), false)?;
    }
    Ok(())
}

#[test]
fn profile_cli_non_regular_input_has_a_distinct_failure_without_store_creation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("never-opened");
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &[
                "profile",
                "register",
                "--file",
                path_text(directory.path())?,
            ],
        )?,
        "input_not_regular_file",
    )?;
    equal(root.exists(), false)
}

#[test]
fn profile_cli_checked_semantic_errors_remain_failures_and_do_not_open_the_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("never-opened");
    let file = directory.path().join("profile.json");
    let mut input = profile()?.input().clone();
    input.jurisdictions.clear();
    std::fs::write(&file, serde_json::to_vec(&input)?)?;
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "register", "--file", path_text(&file)?],
        )?,
        "profile_invalid",
    )?;
    equal(root.exists(), false)
}
