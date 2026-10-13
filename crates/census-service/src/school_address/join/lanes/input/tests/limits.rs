use super::*;

#[test]
fn source_pair_and_acquisition_record_limits_are_exact() -> TestResult {
    let mut pairs = BTreeMap::new();
    let mut acquisitions = Vec::new();
    for index in 0..MAX_EVIDENCE_RECORDS {
        let path = format!("capture-{index}.html");
        pairs.insert(format!("state-ed@{path}"), "2026-09-01".to_string());
        acquisitions.push(acquisition(u32::try_from(index)?, &path, None));
    }
    let legacy = Overrides::from_source_pairs(BTreeMap::new(), pairs.clone())?;
    check!(serde_json::from_str::<Overrides>(&serde_json::to_string(&legacy)?).is_ok());
    let current = Overrides::from_acquisitions(acquisitions.clone())?;
    check!(serde_json::from_str::<Overrides>(&serde_json::to_string(&current)?).is_ok());
    pairs.insert("state-ed@extra.html".to_string(), "2026-09-01".to_string());
    check!(Overrides::from_source_pairs(BTreeMap::new(), pairs.clone()).is_err());
    let raw = format!("{{\"dates\":{}}}", serde_json::to_string(&pairs)?);
    check!(serde_json::from_str::<Overrides>(&raw).is_err());
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

#[test]
fn source_pair_maps_have_independent_counts_and_one_combined_byte_budget() -> TestResult {
    let mut urls = BTreeMap::new();
    let mut dates = BTreeMap::new();
    for index in 0..MAX_EVIDENCE_RECORDS {
        let source = format!("state-ed@capture-{index}.html");
        urls.insert(source.clone(), "https://example.org/a".to_string());
        dates.insert(source, "2026-09-01".to_string());
    }
    let overrides = Overrides::from_source_pairs(urls, dates)?;
    check!(serde_json::from_str::<Overrides>(&serde_json::to_string(&overrides)?).is_ok());
    Ok(())
}

#[test]
fn legacy_combined_text_budget_has_exact_constructor_and_serde_boundaries() -> TestResult {
    let url = "https://example.org/ab".to_string();
    let date = "2026-09-01".to_string();
    let value_bytes = url
        .len()
        .checked_add(date.len())
        .ok_or("value byte overflow")?;
    let selector_bytes = MAX_EVIDENCE_TEXT_BYTES
        .checked_sub(value_bytes)
        .and_then(|bytes| bytes.checked_div(2))
        .ok_or("selector byte budget overflow")?;
    let prefix = "state-ed@";
    let padding = selector_bytes
        .checked_sub(prefix.len())
        .ok_or("selector budget smaller than prefix")?;
    let selector = format!("{prefix}{}", "x".repeat(padding));
    let combined_bytes = selector
        .len()
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(value_bytes))
        .ok_or("combined byte overflow")?;
    check!(eq; combined_bytes, MAX_EVIDENCE_TEXT_BYTES);
    let mut urls = BTreeMap::from([(selector.clone(), url)]);
    let dates = BTreeMap::from([(selector.clone(), date)]);
    let accepted = Overrides::from_source_pairs(urls.clone(), dates.clone())?;
    let accepted_raw = serde_json::to_string(&accepted)?;
    let restored: Overrides = serde_json::from_str(&accepted_raw)?;
    check!(restored.acquisitions().is_none());
    check!(eq; serde_json::to_string(&restored)?, accepted_raw);
    urls.get_mut(&selector)
        .ok_or("missing URL selector")?
        .push('x');
    check!(Overrides::from_source_pairs(urls.clone(), BTreeMap::new()).is_ok());
    check!(Overrides::from_source_pairs(BTreeMap::new(), dates.clone()).is_ok());
    check!(Overrides::from_source_pairs(urls.clone(), dates.clone()).is_err());
    let oversized_raw = format!(
        "{{\"urls\":{},\"dates\":{}}}",
        serde_json::to_string(&urls)?,
        serde_json::to_string(&dates)?,
    );
    check!(serde_json::from_str::<Overrides>(&oversized_raw).is_err());
    Ok(())
}
