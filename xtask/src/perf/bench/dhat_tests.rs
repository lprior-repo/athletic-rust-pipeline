use super::{parse, MAX_PROFILE_BYTES, MAX_PROFILE_POINTS};
use anyhow::{anyhow, Result};

fn profile(points: &str) -> String {
    format!(r#"{{"dhatFileVersion":2,"mode":"heap","pps":[{points}]}}"#)
}

fn rejected(raw: &str, expected: &str) -> Result<()> {
    let error = match parse(raw.as_bytes()) {
        Err(error) => format!("{error:#}"),
        Ok(totals) => return Err(anyhow!("invalid profile accepted as {totals:?}")),
    };
    check!(error.contains(expected), "{error}");
    Ok(())
}

#[test]
fn dhat_totals_include_every_point_and_preserve_u64_precision() -> Result<()> {
    let raw = profile(r#"{"tbk":0,"tb":0},{"tbk":2800,"tb":1000000},{"tbk":6,"tb":22425}"#);
    check!(eq; parse(raw.as_bytes())?, (2806, 1022425));
    let raw = profile(r#"{"tbk":18446744073709551615,"tb":18446744073709551615},{"tbk":0,"tb":0}"#);
    check!(eq; parse(raw.as_bytes())?, (u64::MAX, u64::MAX));
    Ok(())
}

#[test]
fn dhat_missing_invalid_and_incompatible_headers_refuse_metrics() -> Result<()> {
    for raw in [
        "",
        "{",
        r#"{"mode":"heap","pps":[{"tbk":1,"tb":1}]}"#,
        r#"{"dhatFileVersion":2,"pps":[{"tbk":1,"tb":1}]}"#,
        r#"{"dhatFileVersion":2,"mode":"heap"}"#,
        r#"{"dhatFileVersion":2,"mode":"heap","pps":null}"#,
        r#"{"dhatFileVersion":2,"mode":"heap","pps":{}}"#,
        r#"{"dhatFileVersion":2.0,"mode":"heap","pps":[{"tbk":1,"tb":1}]}"#,
        r#"{"dhatFileVersion":2,"mode":"heap","pps":[{"tbk":1,"tb":1}]} garbage"#,
        r#"{"dhatFileVersion":2,"dhatFileVersion":2,"mode":"heap","pps":[{"tbk":1,"tb":1}]}"#,
    ] {
        rejected(raw, "invalid DHAT heap profile JSON")?;
    }
    for raw in [
        r#"{"dhatFileVersion":1,"mode":"heap","pps":[{"tbk":1,"tb":1}]}"#,
        r#"{"dhatFileVersion":3,"mode":"heap","pps":[{"tbk":1,"tb":1}]}"#,
        r#"{"dhatFileVersion":2,"mode":"ad-hoc","pps":[{"tbk":1,"tb":1}]}"#,
        r#"{"dhatFileVersion":2,"mode":"copy","pps":[{"tbk":1,"tb":1}]}"#,
    ] {
        rejected(raw, "requires dhatFileVersion 2 and mode heap")?;
    }
    Ok(())
}

#[test]
fn dhat_malformed_later_points_cannot_disappear_from_positive_totals() -> Result<()> {
    for point in [
        r#"{"tbk":1}"#,
        r#"{"tb":1}"#,
        r#"{"tbk":null,"tb":1}"#,
        r#"{"tbk":1,"tb":null}"#,
        r#"{"tbk":-1,"tb":1}"#,
        r#"{"tbk":1,"tb":-1}"#,
        r#"{"tbk":0.5,"tb":1}"#,
        r#"{"tbk":1,"tb":0.5}"#,
        r#"{"tbk":"1","tb":1}"#,
        r#"{"tbk":1,"tb":"1"}"#,
        r#"{"tbk":18446744073709551616,"tb":1}"#,
        r#"{"tbk":1,"tb":18446744073709551616}"#,
        "null",
    ] {
        let raw = profile(&format!(r#"{{"tbk":1,"tb":1}},{point}"#));
        rejected(&raw, "invalid DHAT heap profile JSON")?;
    }
    Ok(())
}

#[test]
fn dhat_empty_or_zero_aggregates_cannot_become_measurements() -> Result<()> {
    for points in [
        "",
        r#"{"tbk":0,"tb":0}"#,
        r#"{"tbk":0,"tb":1},{"tbk":0,"tb":1}"#,
        r#"{"tbk":1,"tb":0},{"tbk":1,"tb":0}"#,
    ] {
        rejected(&profile(points), "must be positive")?;
    }
    Ok(())
}

#[test]
fn dhat_each_aggregate_refuses_overflow_in_a_later_point() -> Result<()> {
    for (points, expected) in [
        (
            r#"{"tbk":18446744073709551615,"tb":1},{"tbk":1,"tb":1}"#,
            "DHAT allocation count overflow",
        ),
        (
            r#"{"tbk":1,"tb":18446744073709551615},{"tbk":1,"tb":1}"#,
            "DHAT allocated bytes overflow",
        ),
    ] {
        rejected(&profile(points), expected)?;
    }
    Ok(())
}

#[test]
fn dhat_point_budget_accepts_the_boundary_and_refuses_one_more_point() -> Result<()> {
    let mut points = r#"{"tbk":1,"tb":1},"#.repeat(MAX_PROFILE_POINTS - 1);
    points.push_str(r#"{"tbk":1,"tb":1}"#);
    let count = u64::try_from(MAX_PROFILE_POINTS)?;
    check!(eq; parse(profile(&points).as_bytes())?, (count, count));
    points.push_str(r#",{"tbk":1,"tb":1}"#);
    rejected(&profile(&points), "DHAT profile exceeds 65536 points")?;
    Ok(())
}

#[test]
fn dhat_byte_budget_includes_ignored_data_and_trailing_whitespace() -> Result<()> {
    let mut raw = profile(r#"{"tbk":1,"tb":1,"frames":[0,1]}"#);
    let budget = usize::try_from(MAX_PROFILE_BYTES)?;
    let padding = budget
        .checked_sub(raw.len())
        .ok_or_else(|| anyhow!("profile exceeds budget"))?;
    raw.push_str(&" ".repeat(padding));
    check!(eq; parse(raw.as_bytes())?, (1, 1));
    raw.push(' ');
    rejected(&raw, "DHAT profile exceeds 16 MiB")?;
    Ok(())
}
