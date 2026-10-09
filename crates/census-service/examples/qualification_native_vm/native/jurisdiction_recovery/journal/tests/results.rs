use super::{settled_sources, support};
use anyhow::{Context, Result};
use serde_json::json;

#[test]
fn source_call_acknowledgement_cannot_name_another_child() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    assert_eq!(
        settled_sources(&original, &observation)?
            .context("control refused")?
            .len(),
        2
    );
    *support::parent_entry(&mut observation, 2)?
        .pointer_mut("/entry_json/Notification/Completion/CallInvocationId/invocation_id")
        .context("acknowledgement absent")? = json!("inv_sibling");
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn successful_result_without_matching_child_acknowledgement_is_not_admitted() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    *support::parent_entry(&mut observation, 2)?
        .pointer_mut("/entry_json/Notification/Completion/CallInvocationId/completion_id")
        .context("acknowledgement absent")? = json!(11);
    assert!(settled_sources(&original, &observation)?.is_none());
    Ok(())
}

#[test]
fn unrelated_result_completion_cannot_settle_the_original_source_call() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    *support::parent_entry(&mut observation, 3)?
        .pointer_mut("/entry_json/Notification/Completion/Call/completion_id")
        .context("result completion absent")? = json!(11);
    assert!(settled_sources(&original, &observation)?.is_none());
    Ok(())
}

#[test]
fn run_completion_with_source_result_id_cannot_replace_a_call_completion() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    let entry = support::parent_entry(&mut observation, 3)?;
    let result = entry
        .pointer("/entry_json/Notification/Completion/Call")
        .context("call result absent")?
        .clone();
    *entry.get_mut("entry_type").context("entry type absent")? = json!("Notification: Run");
    *entry.get_mut("entry_json").context("entry JSON absent")? =
        json!({"Notification":{"Completion":{"Run":result}}});
    assert!(settled_sources(&original, &observation)?.is_none());
    Ok(())
}

#[test]
fn failed_parent_call_result_cannot_certify_a_successful_child_status() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    *support::parent_entry(&mut observation, 3)?
        .pointer_mut("/entry_json/Notification/Completion/Call/result")
        .context("result absent")? = json!({"Failure":{"code":500,"message":"source failed"}});
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn source_call_result_cannot_use_another_childs_settled_outcome() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    let sibling_result = support::parent_entry(&mut observation, 6)?
        .pointer("/entry_json/Notification/Completion/Call/result")
        .context("sibling result absent")?
        .clone();
    *support::parent_entry(&mut observation, 3)?
        .pointer_mut("/entry_json/Notification/Completion/Call/result")
        .context("first result absent")? = sibling_result;
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}

#[test]
fn malformed_success_bytes_do_not_become_a_settled_source_result() -> Result<()> {
    for bytes in [json!([256]), json!([-1]), json!([123])] {
        let (original, mut observation) = support::completed()?;
        *support::parent_entry(&mut observation, 3)?
            .pointer_mut("/entry_json/Notification/Completion/Call/result/Success")
            .context("success payload absent")? = bytes;
        assert!(settled_sources(&original, &observation).is_err());
    }
    Ok(())
}

#[test]
fn duplicate_result_completion_id_is_refused_instead_of_reusing_one_answer() -> Result<()> {
    let (original, mut observation) = support::completed()?;
    *support::parent_entry(&mut observation, 6)?
        .pointer_mut("/entry_json/Notification/Completion/Call/completion_id")
        .context("second result absent")? = json!(8);
    assert!(settled_sources(&original, &observation).is_err());
    Ok(())
}
