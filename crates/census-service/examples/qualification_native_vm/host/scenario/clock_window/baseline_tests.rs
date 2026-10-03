use super::*;
use serde_json::json;

fn captured() -> Result<(Value, Value)> {
    let capture: Value = serde_json::from_str(include_str!("root18.json"))?;
    let injection = capture
        .get("injection")
        .context("captured injection absent")?;
    let baseline = capture
        .get("baseline")
        .context("captured baseline absent")?;
    Ok((injection.clone(), baseline.clone()))
}

#[test]
fn authentic_root18_same_machine_reversion_refuses_baseline() -> Result<()> {
    let (injection, baseline) = captured()?;
    validate_injection(&injection)?;
    validate_machine(
        injection.get("after").context("captured after absent")?,
        &baseline,
    )?;
    assert!(validate_baseline(&injection, &baseline).is_err());
    Ok(())
}

#[test]
fn advancing_pre_midnight_baseline_is_accepted() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:55:02+00:00");
    validate_baseline(&injection, &baseline)
}

#[test]
fn baseline_at_observed_injection_is_accepted() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:55:00.047970997+00:00");
    validate_baseline(&injection, &baseline)
}

#[test]
fn baseline_one_nanosecond_before_injection_is_refused_within_window() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:55:00.047970996+00:00");
    assert!(validate_baseline(&injection, &baseline).is_err());
    Ok(())
}

#[test]
fn baseline_reported_date_cannot_override_actual_realtime_date() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-03T23:55:02+00:00");
    assert!(validate_baseline(&injection, &baseline).is_err());
    Ok(())
}

#[test]
fn baseline_realtime_is_compared_in_utc_not_its_reported_offset() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-03T01:55:02+02:00");
    validate_baseline(&injection, &baseline)
}

#[test]
fn baseline_last_nanosecond_before_midnight_is_accepted() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:59:59.999999999+00:00");
    validate_baseline(&injection, &baseline)
}

#[test]
fn baseline_at_next_midnight_is_refused() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-03T00:00:00+00:00");
    baseline["date"] = json!("2026-10-03");
    assert!(validate_baseline(&injection, &baseline).is_err());
    Ok(())
}

#[test]
fn another_dates_pre_midnight_window_cannot_replace_original_injection() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-03T23:55:00+00:00");
    baseline["date"] = json!("2026-10-03");
    assert!(validate_baseline(&injection, &baseline).is_err());
    Ok(())
}

#[test]
fn baseline_requires_nonempty_unchanged_boot_and_machine() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:55:02+00:00");
    for field in ["boot_id", "machine_id"] {
        for invalid in [json!("different"), json!(""), Value::Null] {
            let mut changed = baseline.clone();
            changed[field] = invalid;
            assert!(validate_baseline(&injection, &changed).is_err());
        }
        let mut missing = baseline.clone();
        missing
            .as_object_mut()
            .context("baseline not an object")?
            .remove(field)
            .context("captured baseline field absent")?;
        assert!(validate_baseline(&injection, &missing).is_err());
    }
    Ok(())
}

#[test]
fn baseline_requires_parseable_actual_realtime_and_reported_date() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:55:02+00:00");
    for field in ["date", "realtime"] {
        for invalid in [json!("invalid"), json!(""), Value::Null, json!(17)] {
            let mut changed = baseline.clone();
            changed[field] = invalid;
            assert!(validate_baseline(&injection, &changed).is_err());
        }
        let mut missing = baseline.clone();
        missing
            .as_object_mut()
            .context("baseline not an object")?
            .remove(field)
            .context("captured baseline field absent")?;
        assert!(validate_baseline(&injection, &missing).is_err());
    }
    Ok(())
}

#[test]
fn baseline_cannot_certify_an_invalid_original_injection() -> Result<()> {
    let (injection, mut baseline) = captured()?;
    baseline["realtime"] = json!("2026-10-02T23:55:02+00:00");
    for (side, field, invalid) in [
        ("before", "date", "invalid"),
        ("before", "date", "2026-10-01"),
        ("before", "boot_id", "different"),
        ("before", "machine_id", "different"),
        ("after", "realtime", "2026-10-02T23:54:59+00:00"),
        ("after", "realtime", "invalid"),
        ("after", "date", "2026-10-01"),
    ] {
        let mut changed = injection.clone();
        changed[side][field] = json!(invalid);
        assert!(validate_baseline(&changed, &baseline).is_err());
    }
    for side in ["before", "after"] {
        let mut missing = injection.clone();
        missing
            .as_object_mut()
            .context("injection not an object")?
            .remove(side)
            .context("captured injection side absent")?;
        assert!(validate_baseline(&missing, &baseline).is_err());
    }
    Ok(())
}

#[test]
fn window_is_derived_from_injection_date_including_year_rollover() -> Result<()> {
    let (mut injection, mut baseline) = captured()?;
    injection["before"]["date"] = json!("2027-12-31");
    injection["after"]["date"] = json!("2027-12-31");
    injection["after"]["realtime"] = json!("2027-12-31T23:55:00Z");
    baseline["date"] = json!("2027-12-31");
    baseline["realtime"] = json!("2027-12-31T23:55:00Z");
    validate_injection(&injection)?;
    validate_baseline(&injection, &baseline)?;
    baseline["date"] = json!("2028-01-01");
    baseline["realtime"] = json!("2028-01-01T00:00:00Z");
    assert!(validate_baseline(&injection, &baseline).is_err());
    Ok(())
}
