use super::*;
use census_crawl::participation_evidence::result_ownership::RetainedResultOwnershipError;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn field_mut<'a>(
    value: &'a mut serde_json::Value,
    field: &str,
) -> TestResult<&'a mut serde_json::Value> {
    value
        .get_mut(field)
        .ok_or_else(|| format!("missing mutable JSON field {field}").into())
}

#[test]
fn corrupted_body_commitment_refuses_before_store_open() -> TestResult {
    let directory = tempfile::tempdir()?;
    let capture = super::tests::fixture_capture()?;
    let locators = super::tests::fixture_locators();
    read_retained_html_result_rows(&capture, &locators)?;

    let mut corrupted = serde_json::to_value(&capture)?;
    *field_mut(&mut corrupted, "body_sha256")? = serde_json::json!("0".repeat(64));

    let capture_path = directory.path().join("capture.json");
    let rows_path = directory.path().join("rows.json");
    std::fs::write(&capture_path, serde_json::to_vec(&corrupted)?)?;
    std::fs::write(&rows_path, serde_json::to_vec(&locators)?)?;

    let root = directory.path().join("store");
    let mut output = Vec::new();
    let error = run_to(
        &root,
        &ResultIntakeArgs {
            capture: capture_path,
            rows: rows_path,
            operation: "corrupt-body-intake".into(),
        },
        &mut output,
    )
    .err()
    .ok_or("corrupted body commitment was accepted")?;

    check!(matches!(
        error.downcast_ref::<RetainedResultOwnershipError>(),
        Some(RetainedResultOwnershipError::Capture(_))
    ));
    check!(output.is_empty());
    check!(!root.exists());
    Ok(())
}

#[test]
fn redirected_capture_url_refuses_before_store_open() -> TestResult {
    let directory = tempfile::tempdir()?;
    let capture = super::tests::fixture_capture()?;
    let locators = super::tests::fixture_locators();
    read_retained_html_result_rows(&capture, &locators)?;

    let mut altered = serde_json::to_value(&capture)?;
    *field_mut(&mut altered, "final_url")? =
        serde_json::json!("https://other.milesplit.com/meets/redirected");

    let capture_path = directory.path().join("capture.json");
    let rows_path = directory.path().join("rows.json");
    std::fs::write(&capture_path, serde_json::to_vec(&altered)?)?;
    std::fs::write(&rows_path, serde_json::to_vec(&locators)?)?;

    let root = directory.path().join("store");
    let mut output = Vec::new();
    let error = run_to(
        &root,
        &ResultIntakeArgs {
            capture: capture_path,
            rows: rows_path,
            operation: "redirected-url-intake".into(),
        },
        &mut output,
    )
    .err()
    .ok_or("redirected capture URL was accepted")?;

    check!(matches!(
        error.downcast_ref::<RetainedResultOwnershipError>(),
        Some(RetainedResultOwnershipError::Capture(_))
    ));
    check!(output.is_empty());
    check!(!root.exists());
    Ok(())
}
