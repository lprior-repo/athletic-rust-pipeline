use super::super::super::super::journal::{settled_sources, tests::support};
use super::admit;
use anyhow::{Context, Result};
use serde_json::json;

#[test]
fn original_already_completed_before_pause_is_admitted_with_source_identities_preserved(
) -> Result<()> {
    let (original, current) = support::completed()?;
    let tree = support::tree(&current)?;
    assert_eq!(admit(&original, &current, &current, &tree, &tree)?, true);
    let sources = settled_sources(&original, &current)?.context("completed sources absent")?;
    assert_eq!(
        sources
            .iter()
            .map(|source| (
                source.call.command.invocation_id.as_str(),
                source.call.child_id.as_str(),
                source.call.request.source.as_str(),
                source.child.id.as_str(),
            ))
            .collect::<Vec<_>>(),
        vec![
            ("inv_parent", "inv_child", "milesplit", "inv_child"),
            ("inv_parent", "inv_sibling", "riil", "inv_sibling"),
        ]
    );
    Ok(())
}

#[test]
fn paused_original_finishing_after_acknowledgement_is_admitted_not_forced_to_remain_paused(
) -> Result<()> {
    let (original, current) = support::completed()?;
    let mut previous = support::active_before(&original)?;
    support::status_field(&mut previous, "inv_parent", "status", json!("paused"))?;
    let tree = support::tree(&current)?;
    assert_eq!(admit(&original, &previous, &current, &tree, &tree)?, true);
    Ok(())
}

#[test]
fn completed_but_failed_original_root_is_refused() -> Result<()> {
    let (original, mut current) = support::completed()?;
    let previous = support::active_before(&original)?;
    support::status_field(
        &mut current,
        "inv_parent",
        "completion_result",
        json!("failure"),
    )?;
    let tree = support::tree(&current)?;
    assert!(admit(&original, &previous, &current, &tree, &tree).is_err());
    Ok(())
}

#[test]
fn completed_but_failed_nested_descendant_is_not_hidden_by_successful_sources() -> Result<()> {
    let (original, current) = support::completed()?;
    let previous = support::active_before(&original)?;
    let mut tree = support::tree(&current)?;
    tree.push(json!({"id":"inv_nested","invoked_by_id":"inv_child",
        "status":"completed","completion_result":"failure"}));
    assert!(admit(&original, &previous, &current, &tree, &tree).is_err());
    Ok(())
}

#[test]
fn original_parent_cannot_change_owner_scope_deployment_or_protocol_during_completion() -> Result<()>
{
    for (field, value) in [
        ("target_service_key", json!("foreign-parent")),
        ("target_service_name", json!("ForeignCensus")),
        ("pinned_deployment_id", json!("dp_foreign")),
        ("pinned_service_protocol_version", json!(6)),
        ("restarted_from", json!("inv_replacement")),
    ] {
        let (original, mut current) = support::completed()?;
        let previous = support::active_before(&original)?;
        support::status_field(&mut current, "inv_parent", field, value)?;
        let tree = support::tree(&current)?;
        assert!(
            admit(&original, &previous, &current, &tree, &tree).is_err(),
            "{field}"
        );
    }
    Ok(())
}

#[test]
fn original_child_protocol_cannot_change_to_another_supported_version() -> Result<()> {
    let (original, mut current) = support::completed()?;
    let previous = support::active_before(&original)?;
    support::status_field(
        &mut current,
        "inv_child",
        "pinned_service_protocol_version",
        json!(6),
    )?;
    let tree = support::tree(&current)?;
    assert!(admit(&original, &previous, &current, &tree, &tree).is_err());
    Ok(())
}

#[test]
fn missing_actual_root_or_source_child_prevents_completed_admission() -> Result<()> {
    for id in ["inv_parent", "inv_child", "inv_sibling"] {
        let (original, current) = support::completed()?;
        let previous = support::active_before(&original)?;
        let mut tree = support::tree(&current)?;
        tree.retain(|row| row.get("id").and_then(serde_json::Value::as_str) != Some(id));
        assert!(
            admit(&original, &previous, &current, &tree, &tree).is_err(),
            "{id}"
        );
    }
    Ok(())
}

#[test]
fn actual_tree_cannot_reparent_an_observed_original_source_child() -> Result<()> {
    let (original, current) = support::completed()?;
    let previous = support::active_before(&original)?;
    let mut tree = support::tree(&current)?;
    let child = tree
        .iter_mut()
        .find(|row| row.get("id") == Some(&json!("inv_child")))
        .context("source child absent")?;
    *child.get_mut("invoked_by_id").context("owner absent")? = json!("inv_sibling");
    assert!(admit(&original, &previous, &current, &tree, &tree).is_err());
    Ok(())
}

#[test]
fn actual_tree_refuses_a_foreign_unretained_descendant_owner() -> Result<()> {
    let (original, current) = support::completed()?;
    let mut tree = support::tree(&current)?;
    tree.push(json!({"id":"inv_foreign","invoked_by_id":"inv_unretained",
        "status":"completed","completion_result":"success"}));
    assert!(admit(&original, &current, &current, &tree, &tree).is_err());
    Ok(())
}

#[test]
fn changed_tree_bookends_are_not_certified_as_completed_recovery() -> Result<()> {
    let (original, current) = support::completed()?;
    let before = support::tree(&current)?;
    let mut after = before.clone();
    *after
        .first_mut()
        .context("source status absent")?
        .get_mut("journal_size")
        .context("journal size absent")? = json!(4);
    assert_eq!(
        admit(&original, &current, &current, &before, &after)?,
        false
    );
    Ok(())
}

#[test]
fn original_child_journal_cannot_lose_acknowledged_registration_when_root_completes() -> Result<()>
{
    let (original, mut current) = support::completed()?;
    let previous = support::active_before(&original)?;
    current
        .children
        .first_mut()
        .context("original child absent")?
        .journal
        .as_array_mut()
        .context("child journal absent")?
        .truncate(1);
    support::status_field(&mut current, "inv_child", "journal_size", json!(1))?;
    let tree = support::tree(&current)?;
    assert!(admit(&original, &previous, &current, &tree, &tree).is_err());
    Ok(())
}
