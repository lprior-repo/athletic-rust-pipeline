use super::ClockProgress;
use anyhow::{Context, Result};
use serde_json::{json, Value};

mod invalid;

fn clock(day: &str, time: &str, uptime: &str) -> Value {
    json!({
        "date": day,
        "realtime": format!("{day}T{time}Z"),
        "boot_id": "boot",
        "machine_id": "machine",
        "monotonic_uptime": uptime
    })
}

#[test]
fn previous_day_and_same_day_backward_samples_cannot_certify_crossing() -> Result<()> {
    let baseline = clock("2026-10-02", "23:55:02", "100.00 0.00");
    for sample in [
        clock("2026-10-01", "23:55:03", "101.00 0.00"),
        clock("2026-10-02", "23:54:59", "101.00 0.00"),
    ] {
        let mut progress = ClockProgress::new(&baseline)?;
        assert!(progress.observe(&sample).is_err());
    }
    Ok(())
}

#[test]
fn backward_between_advanced_samples_is_refused_even_above_original_baseline() -> Result<()> {
    let baseline = clock("2026-10-02", "23:55:02", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    let advanced = clock("2026-10-02", "23:55:04", "102.00 0.00");
    assert_eq!(progress.observe(&advanced)?, false);
    let backward = clock("2026-10-02", "23:55:03.500", "102.50 0.00");
    assert!(progress.observe(&backward).is_err());
    Ok(())
}

#[test]
fn even_one_nanosecond_of_sampled_realtime_regression_is_refused() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:58", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert_eq!(
        progress.observe(&clock("2026-10-02", "23:59:59", "101.00 0.00"))?,
        false
    );
    let backward = clock("2026-10-02", "23:59:58.999999999", "101.00 0.00");
    assert!(progress.observe(&backward).is_err());
    Ok(())
}

#[test]
fn immediate_and_delayed_forward_steps_require_matching_monotonic_elapsed() -> Result<()> {
    let baseline = clock("2026-10-02", "23:55:02", "100.00 0.00");
    let mut immediate = ClockProgress::new(&baseline)?;
    assert!(immediate
        .observe(&clock("2026-10-03", "00:00:01", "102.00 0.00"))
        .is_err());
    let mut delayed = ClockProgress::new(&baseline)?;
    assert_eq!(
        delayed.observe(&clock("2026-10-02", "23:56:02", "160.00 0.00"))?,
        false
    );
    assert!(delayed
        .observe(&clock("2026-10-03", "00:00:01", "161.00 0.00"))
        .is_err());
    Ok(())
}

#[test]
fn gradual_tolerated_steps_cannot_accumulate_past_original_baseline_allowance() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:57", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert_eq!(
        progress.observe(&clock("2026-10-02", "23:59:58.400", "101.00 0.00"))?,
        false
    );
    assert_eq!(
        progress.observe(&clock("2026-10-02", "23:59:59.800", "102.00 0.00"))?,
        false
    );
    assert!(progress
        .observe(&clock("2026-10-03", "00:00:01.200", "103.00 0.00"))
        .is_err());
    Ok(())
}

#[test]
fn adjacent_elapsed_discrepancy_is_refused_even_when_original_elapsed_agrees() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:57", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert_eq!(
        progress.observe(&clock("2026-10-02", "23:59:58", "101.80 0.00"))?,
        false
    );
    assert!(progress
        .observe(&clock("2026-10-02", "23:59:59.600", "102.00 0.00"))
        .is_err());
    Ok(())
}

#[test]
fn ordinary_advance_from_retained_root10_baseline_certifies_the_successor_date() -> Result<()> {
    let capture: Value = serde_json::from_str(include_str!("../tests/root10.json"))?;
    let baseline = capture
        .get("baseline")
        .context("captured baseline absent")?;
    let mut progress = ClockProgress::new(baseline)?;
    let mut sample = baseline.clone();
    sample["realtime"] = json!("2026-10-02T23:56:02.218815469Z");
    sample["monotonic_uptime"] = json!("110.86 44.61");
    assert_eq!(progress.observe(&sample)?, false);
    sample["date"] = json!("2026-10-03");
    sample["realtime"] = json!("2026-10-03T00:00:00.218815469Z");
    sample["monotonic_uptime"] = json!("348.86 44.61");
    assert_eq!(progress.observe(&sample)?, true);
    Ok(())
}

#[test]
fn offset_and_year_rollover_are_compared_in_utc() -> Result<()> {
    let baseline = clock("2027-12-31", "23:59:59", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    let mut sample = clock("2028-01-01", "00:00:00", "101.00 500.00");
    sample["realtime"] = json!("2028-01-01T02:00:00+02:00");
    assert_eq!(progress.observe(&sample)?, true);
    Ok(())
}

#[test]
fn two_decimal_uptime_quantization_does_not_require_a_one_second_elapsed_floor() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:59.999999999", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert_eq!(
        progress.observe(&clock("2026-10-03", "00:00:00", "100.00 0.00"))?,
        true
    );
    Ok(())
}

#[test]
fn one_second_total_measurement_allowance_is_inclusive_and_symmetric() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:58", "100.00 0.00");
    for uptime in ["101.00 0.00", "103.00 0.00"] {
        let mut progress = ClockProgress::new(&baseline)?;
        assert_eq!(
            progress.observe(&clock("2026-10-03", "00:00:00", uptime))?,
            true
        );
    }
    for sample in [
        clock("2026-10-03", "00:00:00.000000001", "101.00 0.00"),
        clock("2026-10-03", "00:00:00", "103.01 0.00"),
    ] {
        let mut progress = ClockProgress::new(&baseline)?;
        assert!(progress.observe(&sample).is_err());
    }
    Ok(())
}

#[test]
fn uptime_cannot_regress_from_baseline_or_between_advanced_samples() -> Result<()> {
    let baseline = clock("2026-10-02", "23:55:02", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert!(progress
        .observe(&clock("2026-10-02", "23:55:02.100", "99.99 0.00"))
        .is_err());
    assert_eq!(
        progress.observe(&clock("2026-10-02", "23:55:04", "102.00 0.00"))?,
        false
    );
    assert!(progress
        .observe(&clock("2026-10-02", "23:55:04.100", "101.99 0.00"))
        .is_err());
    Ok(())
}

#[test]
fn refused_sample_does_not_replace_previous_validated_observation() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:58", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert_eq!(
        progress.observe(&clock("2026-10-02", "23:59:59", "101.00 0.00"))?,
        false
    );
    assert!(progress
        .observe(&clock("2026-10-03", "00:00:03", "102.00 0.00"))
        .is_err());
    assert_eq!(
        progress.observe(&clock("2026-10-03", "00:00:00", "102.00 0.00"))?,
        true
    );
    Ok(())
}

#[test]
fn elapsed_agreement_cannot_authorize_skipping_a_calendar_day() -> Result<()> {
    let baseline = clock("2026-10-02", "23:59:59", "100.00 0.00");
    let mut progress = ClockProgress::new(&baseline)?;
    assert!(progress
        .observe(&clock("2026-10-04", "00:00:00", "86501.00 0.00"))
        .is_err());
    Ok(())
}
