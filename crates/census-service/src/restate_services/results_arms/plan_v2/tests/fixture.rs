use super::*;
use census_domain::model::{
    AcquisitionDigest, CaptureCacheIntent, CensusRun, HistoryWindow, ResultsCaptureScopeV1,
    SchoolYear, SelectedHistoryV1, SourceAdmissionV1, StoreIdentity,
};
use census_domain::UsJurisdiction;

pub(super) fn binding() -> TestResult<ResultsPlanBinding> {
    let identity = StoreIdentity::parse(&"a".repeat(64))?;
    let history = HistoryWindow::new(2026, 2026, "2026-10-14")?;
    let scope = ResultsCaptureScopeV1::new(
        "mi-2026-r1",
        UsJurisdiction::Michigan,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("fixture run")?,
        AcquisitionDigest::parse(&"b".repeat(64))?,
        SelectedHistoryV1::new(history, 2026)?,
        SourceAdmissionV1::milesplit_results(&"c".repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?;
    Ok(ResultsPlanBinding::new(
        identity,
        scope,
        "2026-10-14".to_owned(),
    )?)
}

pub(super) fn empty() -> TestResult<ResultsPlanV2> {
    let usage = census_store::AcquisitionReadBudget::for_selection().selection_usage();
    Ok(PlanBuilder::new(
        binding()?,
        vec!["milesplit".to_owned()],
        Vec::new(),
        Vec::new(),
    )?
    .finish(SelectionUsage::observed(&usage)?)?)
}

pub(super) fn full() -> TestResult<ResultsPlanV2> {
    let empty = empty()?;
    let overhead = encode(&empty)?.len();
    let available = MAX_PLAN_BYTES
        .checked_sub(overhead)
        .ok_or("fixture overhead")?;
    let full_length = MAX_TEXT_BYTES
        .checked_add(3)
        .ok_or("fixture entry length")?;
    let total = available.checked_add(1).ok_or("fixture total")?;
    let count = total
        .checked_add(full_length.checked_sub(1).ok_or("fixture divisor")?)
        .ok_or("fixture count")?
        / full_length;
    let mut text_bytes = total
        .checked_sub(count.checked_mul(3).ok_or("fixture overhead")?)
        .ok_or("fixture text bytes")?;
    let mut pending = Vec::new();
    for index in 0..count {
        let later = count
            .checked_sub(index)
            .and_then(|n| n.checked_sub(1))
            .ok_or("fixture later")?;
        let minimum = later.checked_mul(8).ok_or("fixture minimum")?;
        let length = text_bytes
            .checked_sub(minimum)
            .ok_or("fixture available")?
            .min(MAX_TEXT_BYTES);
        pending.push(locator(index, length)?);
        text_bytes = text_bytes.checked_sub(length).ok_or("fixture remaining")?;
    }
    check!(eq; text_bytes, 0);
    let usage = census_store::AcquisitionReadBudget::for_selection().selection_usage();
    Ok(PlanBuilder::new(
        binding()?,
        vec!["milesplit".to_owned()],
        Vec::new(),
        pending,
    )?
    .finish(SelectionUsage::observed(&usage)?)?)
}

fn locator(index: usize, length: usize) -> TestResult<String> {
    let mut text = format!("{index:08}");
    let padding = length
        .checked_sub(text.len())
        .ok_or("fixture locator length")?;
    text.push_str(&"x".repeat(padding));
    Ok(text)
}

pub(super) fn mutate(field: &str, replacement: serde_json::Value) -> TestResult<Bytes> {
    let plan = empty()?;
    let mut value = serde_json::to_value(&plan)?;
    let object = value.as_object_mut().ok_or("fixture object")?;
    object.insert(field.to_owned(), replacement);
    Ok(Bytes::from(serde_json::to_vec(&value)?))
}
