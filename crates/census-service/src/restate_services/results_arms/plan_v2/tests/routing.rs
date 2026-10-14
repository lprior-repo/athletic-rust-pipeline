use super::*;
use census_domain::model::{AcquisitionSlotV1, AcquisitionWorkId, SourceMeetRef};
use census_domain::UsJurisdiction;

fn meet(id: &str) -> SourceMeetRef {
    SourceMeetRef {
        id: id.to_owned(),
        source: "milesplit".to_owned(),
        source_meet_id: id.to_owned(),
        jurisdiction: UsJurisdiction::Michigan,
        season: "xc".to_owned(),
        year: 2026,
        name: "fixture".to_owned(),
        date: Some("2026-10-01".to_owned()),
        venue: String::new(),
        results_url: format!("https://mi.milesplit.com/meets/{id}/results"),
        observed_on: "2026-10-14".to_owned(),
    }
}

fn rejected() -> TestResult<ResultsPlanV2> {
    let mut builder = PlanBuilder::new(
        fixture::binding()?,
        vec!["milesplit".to_owned()],
        vec![meet("0001")],
        Vec::new(),
    )?;
    builder.record_meet(
        Discovery::Failed(DiscoveryFailure {
            category: DiscoveryFailureCategory::TargetMismatch,
            detail: "invalid structured meet ID".to_owned(),
        }),
        MeetRoute::NotOwned,
    )?;
    let usage = census_store::AcquisitionReadBudget::for_selection().selection_usage();
    Ok(builder.finish(SelectionUsage::observed(&usage)?)?)
}

#[test]
fn sdk_preserves_actual_shared_positive_id_refusal() -> TestResult {
    let plan = rejected()?;
    let replay = decode(&mut encode(&plan)?)?;
    check!(eq; replay.routes, vec![MeetRoute::NotOwned]);
    check!(eq; replay.meets.len(), 1);
    check!(replay.targets.is_empty());
    Ok(())
}

#[test]
fn sdk_cannot_label_derivable_target_as_not_owned() -> TestResult {
    let mut plan = rejected()?;
    *plan.meets.first_mut().ok_or("fixture meet")? = meet("1");
    let mut raw = Bytes::from(serde_json::to_vec(&plan)?);
    invalid(decode(&mut raw), "derivable target lacks owned route")
}

#[test]
fn sdk_rejects_existing_target_with_no_historical_roots() -> TestResult {
    let binding = fixture::binding()?;
    let slot = AcquisitionSlotV1::new(
        binding.identity().clone(),
        binding.scope().owner_key(),
        2026,
        UsJurisdiction::Michigan,
        "1",
    )?;
    let mut builder = PlanBuilder::new(
        binding,
        vec!["milesplit".to_owned()],
        vec![meet("1")],
        Vec::new(),
    )?;
    let target = builder.record_target(
        slot.id()?,
        AcquisitionWorkId::parse(&"d".repeat(64))?,
        census_store::ExistingRecoveryRoute::Unregistered,
    )?;
    builder.record_meet(
        Discovery::Failed(DiscoveryFailure {
            category: DiscoveryFailureCategory::Fetch,
            detail: "fixture unavailable".to_owned(),
        }),
        MeetRoute::Owned(target),
    )?;
    let usage = census_store::AcquisitionSelectionUsage::new(0, 0, 0, 0, 1, 1)?;
    let mut plan = builder.finish(SelectionUsage::observed(&usage)?)?;
    plan.targets.first_mut().ok_or("fixture target")?.selection =
        TargetSelection::ExistingNoPrepared;
    let mut value = serde_json::to_value(&plan)?;
    let usage = value
        .get_mut("selection_usage")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture usage")?;
    usage.insert("absent_target_probes".to_owned(), serde_json::json!(0));
    invalid(
        decode(&mut Bytes::from(serde_json::to_vec(&value)?)),
        "selection usage ranges",
    )
}

#[test]
fn actual_stage_rebinding_includes_original_observation_date() -> TestResult {
    let binding = fixture::binding()?;
    let scope = crate::restate_services::history_stage::HistoricalStageScope {
        jurisdiction: UsJurisdiction::Michigan,
        year: 2026,
        refresh: false,
        window: binding.scope().history(),
        observed_on: chrono::NaiveDate::from_ymd_opt(2026, 10, 14).ok_or("fixture date")?,
    };
    binding.require_stage(scope)?;
    let changed = crate::restate_services::history_stage::HistoricalStageScope {
        observed_on: chrono::NaiveDate::from_ymd_opt(2026, 10, 13).ok_or("fixture date")?,
        ..scope
    };
    check!(matches!(
        binding.require_stage(changed),
        Err(ResultsPlanError::Invalid("actual stage binding"))
    ));
    Ok(())
}
