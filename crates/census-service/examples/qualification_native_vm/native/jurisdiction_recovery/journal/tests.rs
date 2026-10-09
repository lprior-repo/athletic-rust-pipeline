use super::settled_sources;
use anyhow::{Context, Result};
use serde_json::json;

mod identity;
mod results;
pub(in super::super) mod support;

#[test]
fn completed_sources_preserve_original_calls_children_and_source_units() -> Result<()> {
    let (original, observation) = support::completed()?;
    let sources = settled_sources(&original, &observation)?.context("settled sources refused")?;
    let identities = sources
        .iter()
        .map(|source| {
            (
                source.call.command.invocation_id.as_str(),
                source.call.child_id.as_str(),
                source.call.child_key.as_str(),
                source.call.request.source.as_str(),
                source.child.id.as_str(),
                source.status.get("invoked_by_id"),
            )
        })
        .collect::<Vec<_>>();
    let first_key = format!("{}/teams/milesplit", original.key);
    let second_key = format!("{}/teams/riil", original.key);
    let parent = json!(original.id);
    assert_eq!(
        identities,
        vec![
            (
                "inv_parent",
                "inv_child",
                first_key.as_str(),
                "milesplit",
                "inv_child",
                Some(&parent)
            ),
            (
                "inv_parent",
                "inv_sibling",
                second_key.as_str(),
                "riil",
                "inv_sibling",
                Some(&parent)
            ),
        ]
    );
    Ok(())
}

#[test]
fn paused_original_parent_can_admit_its_completed_sources() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    support::status_field(&mut observation, "inv_parent", "status", json!("paused"))?;
    let sources = settled_sources(&original, &observation)?.context("paused sources refused")?;
    assert_eq!(
        sources
            .iter()
            .map(|source| source.call.child_id.as_str())
            .collect::<Vec<_>>(),
        vec!["inv_child", "inv_sibling"]
    );
    Ok(())
}

#[test]
fn incomplete_parent_journal_cannot_admit_source_results() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    observation
        .parent_journal
        .as_array_mut()
        .context("journal absent")?
        .pop()
        .context("completion absent")?;
    assert!(settled_sources(&original, &observation)?.is_none());
    Ok(())
}

#[test]
fn failed_completed_child_does_not_admit_source_results() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    support::status_field(
        &mut observation,
        "inv_sibling",
        "completion_result",
        json!("failure"),
    )?;
    assert!(settled_sources(&original, &observation)?.is_none());
    Ok(())
}
