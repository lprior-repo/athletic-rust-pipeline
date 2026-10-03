use super::{clock, ClockProgress};
use anyhow::{Context, Result};
use serde_json::{json, Value};

#[test]
fn malformed_or_nonfinite_uptime_cannot_enter_baseline_or_sample_certificate() -> Result<()> {
    let baseline = clock("2026-10-02", "23:55:02", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    for raw in [
        "",
        "   ",
        "NaN 0.00",
        "inf 0.00",
        "-1.00 0.00",
        "+101.00 0.00",
        "1e2 0.00",
        "101.00",
        "101.00 nope",
        "101.00 NaN",
        "101.00 -1.00",
        "101.00 0.00 extra",
        "101.0 0.00",
        "101.000 0.00",
        ".00 0.00",
        "101..00 0.00",
        "18446744073709551616.00 0.00",
        "101.00 18446744073709551616.00",
        "101.００ 0.00",
    ] {
        let invalid = clock("2026-10-02", "23:55:03", raw);
        assert!(ClockProgress::new(&invalid).is_err(), "{raw}");
        assert!(progress.observe(&invalid).is_err(), "{raw}");
    }
    let oversized = clock("2026-10-02", "23:55:03", &"0".repeat(65));
    assert!(ClockProgress::new(&oversized).is_err());
    assert!(progress.observe(&oversized).is_err());
    let valid = clock("2026-10-02", "23:55:03", "101.00 500.00");
    assert_eq!(progress.observe(&valid)?, false);
    Ok(())
}

#[test]
fn missing_null_or_nonstring_fields_cannot_certify_clock_progress() -> Result<()> {
    let baseline = clock("2026-10-02", "23:55:02", "100.00 0.00");
    for field in [
        "date",
        "realtime",
        "monotonic_uptime",
        "boot_id",
        "machine_id",
    ] {
        for invalid in [Value::Null, json!(17), json!("")] {
            let mut value = clock("2026-10-02", "23:55:03", "101.00 0.00");
            value[field] = invalid;
            assert!(ClockProgress::new(&value).is_err(), "{field}");
            let mut progress = ClockProgress::new(&baseline)?;
            assert!(progress.observe(&value).is_err(), "{field}");
        }
        let mut missing = clock("2026-10-02", "23:55:03", "101.00 0.00");
        missing
            .as_object_mut()
            .context("clock not an object")?
            .remove(field)
            .context("clock field absent")?;
        assert!(ClockProgress::new(&missing).is_err(), "{field}");
        let mut progress = ClockProgress::new(&baseline)?;
        assert!(progress.observe(&missing).is_err(), "{field}");
    }
    let mut progress = ClockProgress::new(&baseline)?;
    assert!(ClockProgress::new(&Value::Null).is_err());
    assert!(progress.observe(&Value::Null).is_err());
    Ok(())
}

#[test]
fn changed_boot_or_machine_cannot_certify_even_elapsed_consistent_crossing() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:59", "100.00 0.00");
    for field in ["boot_id", "machine_id"] {
        let mut progress = ClockProgress::new(&baseline)?;
        let mut different = clock("2026-10-03", "00:00:00", "101.00 0.00");
        different[field] = json!("other");
        assert!(progress.observe(&different).is_err());
        let unchanged = clock("2026-10-03", "00:00:00", "101.00 0.00");
        assert_eq!(progress.observe(&unchanged)?, true);
    }
    Ok(())
}

#[test]
fn malformed_realtime_or_dishonest_reported_date_cannot_certify_crossing() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:59", "100.00 0.00");
    for (field, invalid) in [
        ("realtime", "not-an-instant"),
        ("date", "not-a-date"),
        ("date", "2026-10-02"),
        ("realtime", "2026-10-03T02:00:00Z"),
    ] {
        let mut progress = ClockProgress::new(&baseline)?;
        let mut sample = clock("2026-10-03", "00:00:00", "101.00 0.00");
        sample[field] = json!(invalid);
        assert!(progress.observe(&sample).is_err());
    }
    Ok(())
}
