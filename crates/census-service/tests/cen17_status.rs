#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::athleticlive::parse_event_document;
use census_domain::model::{CentiMetres, EventKind, ExactSeconds, Mark};
use serde::Deserialize;
use serde_json::{json, Value};

#[path = "cen17_status/artifacts.rs"]
mod artifacts;
#[path = "cen17_status/athleticnet.rs"]
mod athleticnet;
#[path = "cen17_status/fixtures.rs"]
mod fixtures;

use fixtures::{captured, cases, challenge, control_row, set, CAPTURES, FIELD, XC};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct StatusCase {
    name: String,
    display: Option<String>,
    validity: Value,
    raw: String,
}

#[test]
fn published_invalid_status_overrides_stale_positive_numbers_in_owned_result_captures() -> TestResult
{
    let mut bytes = Vec::new();
    for capture in CAPTURES {
        for case in cases()? {
            let body = challenge(capture, case.display, case.validity)?;
            bytes.clear();
            serde_json::to_writer(&mut bytes, &body)?;
            let doc = parse_event_document("cen17-owned-capture", std::str::from_utf8(&bytes)?)?;
            let row = doc.rows.first().ok_or("missing challenger row")?;
            check!(eq;
                row.canonical_mark(&doc.kind()),
                Some(Mark::Raw(case.raw)),
                "{} {:?}", capture, case.name
            );
        }
    }
    Ok(())
}

#[test]
fn captured_numeric_controls_preserve_exact_time_field_attempts_and_split_channels() -> TestResult {
    let xc = captured(XC)?;
    let field = captured(FIELD)?;
    let xc_doc = parse_event_document("captured-xc", &xc.to_string())?;
    let field_doc = parse_event_document("captured-field", &field.to_string())?;
    let time = xc_doc
        .rows
        .iter()
        .find(|row| row.mark.as_deref() == Some("19:10.5"))
        .ok_or("captured sophomore time")?;
    let height = field_doc.rows.first().ok_or("captured height")?;
    check!(eq; time.canonical_mark(&xc_doc.kind()),
        Some(Mark::TimeSeconds(ExactSeconds::parse("1150.5")?)));
    check!(eq; height.canonical_mark(&field_doc.kind()), Some(Mark::FieldImperial {
        feet_mark: "5-02.00".to_string(), metres: CentiMetres::new(157),
    }));
    check!(eq; time.splits, vec![
        json!({"sp":"6:05.3","cs":"6:05.3"}),
        json!({"sp":"6:16.9","cs":"12:22.2"}),
        json!({"sp":"6:48.3","cs":"19:10.5"}),
    ]);
    Ok(())
}

#[test]
fn absent_display_uses_exact_integer_channel_without_rounding() -> TestResult {
    let mut source = captured(XC)?;
    let mut row = control_row(&mut source)?;
    row.as_object_mut()
        .ok_or("row object")?
        .remove("m")
        .ok_or("published mark")?;
    row.as_object_mut()
        .ok_or("row object")?
        .remove("vm")
        .ok_or("validity")?;
    set(&mut source, "/_source/r", Value::Array(vec![row]))?;
    let doc = parse_event_document("integer-only-capture", &source.to_string())?;
    let row = doc.rows.first().ok_or("row")?;
    let mark = row.canonical_mark(&doc.kind()).ok_or("integer-only time")?;
    let Mark::TimeSeconds(time) = mark else {
        return Err("integer-only time must be numeric".into());
    };
    check!(eq; time.value(), 1_150_500_000_000);
    check!(eq; time.precision(), 3);
    Ok(())
}

#[test]
fn relay_total_is_not_replaced_by_seed_or_individual_split_time() -> TestResult {
    let mut source = captured(XC)?;
    let mut row = control_row(&mut source)?;
    set(&mut row, "/m", json!("3:40.123"))?;
    set(&mut row, "/im", json!(220_123))?;
    set(&mut row, "/s", json!("1:00.000"))?;
    set(&mut row, "/irs", json!([{"sp":"0:50.001","cs":"0:50.001"}]))?;
    set(&mut source, "/_source/ab", json!("4x400m"))?;
    set(&mut source, "/_source/un", json!("4x400m"))?;
    set(&mut source, "/_source/xc", json!(false))?;
    set(&mut source, "/_source/r", Value::Array(vec![row]))?;
    let doc = parse_event_document("relay-captured-schema-challenger", &source.to_string())?;
    check!(eq; doc.kind(), EventKind::Relay4x400);
    let row = doc.rows.first().ok_or("relay row")?;
    check!(eq; row.canonical_mark(&doc.kind()),
        Some(Mark::TimeSeconds(ExactSeconds::parse("220.123")?)));
    Ok(())
}
