use super::{settled_sources, support};
use anyhow::{Context, Result};
use census_service::restate_services::TeamsSourceRequest;
use serde_json::{json, Value};

#[test]
fn duplicate_child_a_cannot_replace_original_child_b_with_equal_cardinality() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    assert_eq!(
        settled_sources(&original, &observation)?
            .context("control refused")?
            .len(),
        2
    );
    let duplicate = serde_json::from_value(serde_json::to_value(
        observation.children.first().context("first child absent")?,
    )?)?;
    *observation
        .children
        .get_mut(1)
        .context("second child absent")? = duplicate;
    assert_eq!(observation.children.len(), 2);
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn distinct_invocations_cannot_repeat_the_same_source_unit_identity() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    let key = observation
        .children
        .first()
        .context("first child absent")?
        .key
        .clone();
    observation
        .children
        .get_mut(1)
        .context("second child absent")?
        .key = key;
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn parent_calls_cannot_repeat_a_source_identity_under_distinct_child_ids() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    let first = support::parent_entry(&mut observation, 1)?.clone();
    let second = support::parent_entry(&mut observation, 4)?;
    for field in ["invocation_target", "parameter"] {
        let pointer = format!("/entry_json/Command/Call/request/{field}");
        *second
            .pointer_mut(&pointer)
            .context("second call field absent")? = first
            .pointer(&pointer)
            .context("first call field absent")?
            .clone();
    }
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn missing_original_child_is_refused_even_when_both_parent_results_exist() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    observation.children.pop().context("second child absent")?;
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn original_child_cannot_change_parent_target_deployment_or_source_unit() -> Result<()> {
    for (field, value) in [
        ("invoked_by_id", json!("inv_foreign")),
        ("target_service_key", json!("foreign/teams/riil")),
        ("target_service_name", json!("ForeignSource")),
        ("target_handler_name", json!("other")),
        ("pinned_deployment_id", json!("dp_foreign")),
        ("restarted_from", json!("inv_replaced")),
    ] {
        let (original, mut observation) = support::completed()?;
        support::status_field(&mut observation, "inv_sibling", field, value)?;
        assert!(settled_sources(&original, &observation).is_err(), "{field}");
    }
    Ok(())
}

#[test]
fn child_input_must_equal_the_original_parent_source_call() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    let request = TeamsSourceRequest {
        jurisdiction: original.request.clone(),
        source: "riil".to_owned(),
        observed_on: "2026-10-03".to_owned(),
    };
    let child = observation
        .children
        .get_mut(1)
        .context("second child absent")?;
    *child
        .journal
        .pointer_mut("/0/entry_json/Command/Input/payload")
        .context("child Input absent")? = json!(serde_json::to_vec(&request)?);
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn foreign_child_journal_cannot_prove_original_completion() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    let child = observation
        .children
        .get_mut(1)
        .context("second child absent")?;
    *child
        .journal
        .pointer_mut("/0/id")
        .context("child journal identity absent")? = json!("inv_foreign");
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn duplicate_status_cannot_hide_a_missing_original_child_status() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    for statuses in [&mut observation.before, &mut observation.after] {
        let rows = statuses.as_array_mut().context("statuses absent")?;
        let duplicate = rows.first().context("first status absent")?.clone();
        *rows.get_mut(1).context("second status absent")? = duplicate;
    }
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn extra_status_cannot_be_dropped_from_the_original_source_census() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    for statuses in [&mut observation.before, &mut observation.after] {
        statuses
            .as_array_mut()
            .context("statuses absent")?
            .push(json!({"id":"inv_orphan"}));
    }
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn incomplete_child_journal_does_not_admit_a_settled_inspection() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    observation
        .children
        .get_mut(1)
        .context("second child absent")?
        .journal = Value::Array(Vec::new());
    assert!(settled_sources(&original, &observation)?.is_none());
    Ok(())
}
