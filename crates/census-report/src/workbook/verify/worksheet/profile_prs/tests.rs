use super::{append_expected, expected_detailed, Value};
use crate::report::ReportError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GROUP_LIMIT_ERROR: &str =
    "complete grouped source-declared PR claims exceed Excel cell budget; no claims were truncated";

#[test]
fn expected_detailed_returns_some_empty_when_there_are_no_lines() -> TestResult {
    check!(eq; expected_detailed(std::iter::empty::<String>())?, Some(Value::Empty));
    Ok(())
}

#[test]
fn expected_detailed_preserves_a_single_complete_line() -> TestResult {
    let expected = "100m: 10.00; Source-declared PR".to_owned();
    check!(eq; expected_detailed([expected.clone()].into_iter())?,
        Some(Value::Text(expected)));
    Ok(())
}

#[test]
fn expected_detailed_sorts_unique_lines_without_changing_their_bytes() -> TestResult {
    let lines = ["200m: 20.00", "100m: 10.00", "200m: 20.00"];
    check!(eq; expected_detailed(lines.into_iter().map(str::to_owned))?,
        Some(Value::Text("100m: 10.00\n200m: 20.00".to_owned())));
    Ok(())
}

#[test]
fn expected_detailed_preserves_exactly_32767_ascii_units() -> TestResult {
    let expected = "a".repeat(32_767);
    check!(eq; expected.encode_utf16().count(), 32_767);
    check!(eq; expected_detailed([expected.clone()].into_iter())?,
        Some(Value::Text(expected)));
    Ok(())
}

#[test]
fn expected_detailed_signals_overflow_at_32768_ascii_units() -> TestResult {
    let line = "a".repeat(32_768);
    check!(eq; line.encode_utf16().count(), 32_768);
    check!(eq; expected_detailed([line].into_iter())?, None);
    Ok(())
}

#[test]
fn expected_detailed_includes_the_separator_in_the_exact_fit_boundary() -> TestResult {
    let first = "a".repeat(16_383);
    let second = "b".repeat(16_383);
    let expected = format!("{first}\n{second}");
    check!(eq; expected.encode_utf16().count(), 32_767);
    check!(eq; expected_detailed([second, first].into_iter())?,
        Some(Value::Text(expected)));
    Ok(())
}

#[test]
fn expected_detailed_refuses_when_the_separator_reaches_32768_units() -> TestResult {
    let first = "a".repeat(16_383);
    let second = "b".repeat(16_384);
    check!(eq; format!("{first}\n{second}").encode_utf16().count(), 32_768);
    check!(eq; expected_detailed([first, second].into_iter())?, None);
    Ok(())
}

#[test]
fn expected_detailed_preserves_supplementary_unicode_at_32767_units() -> TestResult {
    let expected = format!("{}a", "🙂".repeat(16_383));
    check!(eq; expected.encode_utf16().count(), 32_767);
    check!(ne; expected.len(), 32_767);
    check!(eq; expected_detailed([expected.clone()].into_iter())?,
        Some(Value::Text(expected)));
    Ok(())
}

#[test]
fn expected_detailed_counts_supplementary_unicode_as_two_units_at_overflow() -> TestResult {
    let line = "🙂".repeat(16_384);
    check!(eq; line.encode_utf16().count(), 32_768);
    check!(eq; expected_detailed([line].into_iter())?, None);
    Ok(())
}

#[test]
fn expected_detailed_does_not_charge_a_duplicate_of_a_boundary_line() -> TestResult {
    let expected = "a".repeat(32_767);
    check!(eq; expected_detailed([expected.clone(), expected.clone()].into_iter())?,
        Some(Value::Text(expected)));
    Ok(())
}

#[test]
fn expected_detailed_stops_consuming_at_the_first_unique_overflow() -> TestResult {
    let full = "a".repeat(32_766);
    let mut lines = [
        full.clone(),
        full,
        "b".to_owned(),
        "untouched tail".to_owned(),
    ]
    .into_iter();
    let mut consumed_lengths = Vec::new();
    let result = expected_detailed(
        lines
            .by_ref()
            .inspect(|line| consumed_lengths.push(line.len())),
    )?;
    check!(eq; result, None);
    check!(eq; consumed_lengths.len(), 3);
    check!(eq; consumed_lengths, vec![32_766, 32_766, 1]);
    let tail = lines.next().ok_or("overflow consumed the remaining tail")?;
    check!(eq; tail, "untouched tail");
    check!(eq; lines.next(), None);
    Ok(())
}

#[test]
fn append_expected_preserves_an_initial_32767_unit_line_exactly() -> TestResult {
    let expected = "a".repeat(32_767);
    let mut text = String::new();
    let mut units = 0_usize;
    append_expected(&mut text, &mut units, &expected)?;
    check!(eq; text, expected);
    check!(eq; units, 32_767);
    Ok(())
}

#[test]
fn append_expected_admits_unicode_and_separator_at_exactly_32767_units() -> TestResult {
    let mut text = "a".repeat(32_764);
    let expected = format!("{text}\n🙂");
    check!(eq; expected.encode_utf16().count(), 32_767);
    let mut units = 32_764_usize;
    append_expected(&mut text, &mut units, "🙂")?;
    check!(eq; text, expected);
    check!(eq; units, 32_767);
    Ok(())
}

#[test]
fn append_expected_refuses_32768_units_without_changing_text_or_units() -> TestResult {
    let mut text = "a".repeat(32_765);
    let before = text.clone();
    check!(eq; format!("{text}\n🙂").encode_utf16().count(), 32_768);
    let mut units = 32_765_usize;
    let error = append_expected(&mut text, &mut units, "🙂")
        .err()
        .ok_or("32768-unit grouped append_expected succeeded")?;
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == GROUP_LIMIT_ERROR));
    check!(eq; text, before);
    check!(eq; units, 32_765);
    Ok(())
}

#[test]
fn append_expected_refuses_an_initial_32768_unit_line_without_mutation() -> TestResult {
    let mut text = String::new();
    let mut units = 0_usize;
    let error = append_expected(&mut text, &mut units, &"a".repeat(32_768))
        .err()
        .ok_or("32768-unit initial grouped append_expected succeeded")?;
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == GROUP_LIMIT_ERROR));
    check!(eq; text, String::new());
    check!(eq; units, 0);
    Ok(())
}

#[test]
fn append_expected_preserves_duplicate_rows_and_source_order() -> TestResult {
    let mut text = String::new();
    let mut units = 0_usize;
    for line in ["same", "same", "last"] {
        append_expected(&mut text, &mut units, line)?;
    }
    check!(eq; text, "same\nsame\nlast");
    check!(eq; units, 14);
    Ok(())
}
