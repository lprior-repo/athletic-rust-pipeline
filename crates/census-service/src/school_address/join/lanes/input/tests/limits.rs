use super::*;

#[test]
fn acquisition_record_limits_are_exact() -> TestResult {
    let mut acquisitions = Vec::new();
    for index in 0..MAX_EVIDENCE_RECORDS {
        let path = format!("capture-{index}.html");
        acquisitions.push(acquisition(u32::try_from(index)?, &path, None));
    }
    let current = Overrides::from_acquisitions(acquisitions.clone())?;
    check!(serde_json::from_str::<Overrides>(&serde_json::to_string(&current)?).is_ok());
    acquisitions.push(acquisition(
        u32::try_from(MAX_EVIDENCE_RECORDS)?,
        "extra.html",
        None,
    ));
    check!(Overrides::from_acquisitions(acquisitions.clone()).is_err());
    let raw = format!(
        "{{\"acquisitions\":{}}}",
        serde_json::to_string(&acquisitions)?
    );
    check!(serde_json::from_str::<Overrides>(&raw).is_err());
    Ok(())
}

#[test]
fn total_text_budget_rejects_one_byte_overflow_without_truncation() -> TestResult {
    let mut value = acquisition(1, "", None);
    let fixed = value
        .provider
        .len()
        .checked_add(value.url.len())
        .ok_or("fixed byte overflow")?;
    let length = MAX_EVIDENCE_TEXT_BYTES
        .checked_sub(fixed)
        .ok_or("budget smaller than fixed text")?;
    value.path = "x".repeat(length);
    let accepted = Overrides::from_acquisitions(vec![value.clone()])?;
    let raw = serde_json::to_string(&accepted)?;
    check!(serde_json::from_str::<Overrides>(&raw).is_ok());
    value.path.push('x');
    check!(Overrides::from_acquisitions(vec![value]).is_err());
    Ok(())
}
