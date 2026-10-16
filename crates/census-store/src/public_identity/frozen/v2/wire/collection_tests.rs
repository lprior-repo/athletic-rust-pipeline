use super::*;
use crate::public_identity_tests::support::TestResult;

fn array(value: &str, count: usize) -> String {
    format!(
        "[{}]",
        std::iter::repeat_n(value, count)
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn object(rows: &str) -> TestResult<String> {
    let reference = ObjectRefV2::new(
        crate::public_identity::paged_generation::DigestV2::of_bytes(b"fixture"),
        1,
        1,
        4096,
    )?;
    Ok(format!(
        "{{\"reference\":{},\"rows\":{rows}}}",
        serde_json::to_string(&reference)?
    ))
}
fn row() -> &'static str {
    r#"{"key_base64":"YQ==","value_base64":"Yg=="}"#
}

#[test]
fn aggregate_rows_refuse_before_invalid_second_object_body_is_decoded() -> TestResult {
    let first = object(&array(row(), MAX_PHYSICAL_ROWS))?;
    let raw = format!("[{first},{{\"reference\":false,\"rows\":[false]}}]");
    let error = objects(&mut serde_json::Deserializer::from_str(&raw))
        .err()
        .ok_or("aggregate accepted")?;
    check!(error
        .to_string()
        .contains("frozen physical inventory ceiling"));
    Ok(())
}

#[test]
fn physical_row_exact_and_one_over_limits_are_enforced_by_sequence_intake() -> TestResult {
    let exact = array(row(), MAX_PHYSICAL_ROWS);
    check!(eq; physical_rows(&mut serde_json::Deserializer::from_str(&exact))?.len(), MAX_PHYSICAL_ROWS);
    let extra = array(row(), MAX_PHYSICAL_ROWS.checked_add(1).ok_or("bound")?);
    let error = physical_rows(&mut serde_json::Deserializer::from_str(&extra))
        .err()
        .ok_or("rows accepted")?;
    check!(error.to_string().contains("frozen graph inventory ceiling"));
    let oversized_object = object(&extra)?;
    let raw = format!("[{oversized_object}]");
    let error = objects(&mut serde_json::Deserializer::from_str(&raw))
        .err()
        .ok_or("object rows accepted")?;
    check!(error
        .to_string()
        .contains("frozen physical inventory ceiling"));
    Ok(())
}

#[test]
fn objects_and_operations_accept_exact_counts_and_refuse_one_more() -> TestResult {
    let empty_object = object("[]")?;
    let exact = array(&empty_object, MAX_OBJECTS);
    check!(eq; objects(&mut serde_json::Deserializer::from_str(&exact))?.len(), MAX_OBJECTS);
    let extra = array(&empty_object, MAX_OBJECTS.checked_add(1).ok_or("bound")?);
    let error = objects(&mut serde_json::Deserializer::from_str(&extra))
        .err()
        .ok_or("objects accepted")?;
    check!(error.to_string().contains("frozen graph inventory ceiling"));
    let operation = r#"{"sequence":1,"witness":"x","receipt":"x"}"#;
    let exact = array(operation, ReplayLimits::SMALL.records);
    check!(eq; operations(&mut serde_json::Deserializer::from_str(&exact))?.len(), ReplayLimits::SMALL.records);
    let extra = array(
        operation,
        ReplayLimits::SMALL.records.checked_add(1).ok_or("bound")?,
    );
    let error = operations(&mut serde_json::Deserializer::from_str(&extra))
        .err()
        .ok_or("operations accepted")?;
    check!(error.to_string().contains("frozen graph inventory ceiling"));
    Ok(())
}
