use super::{future, unfinished};
use anyhow::{Context, Result};
use serde_json::{json, Value};

const MISSING_COMPLETION: &str =
    "suspended future has no awaited CompletionId; unfinished Sleep correlation refused";

fn root06() -> Result<Value> {
    Ok(serde_json::from_str(include_str!("root06.json"))?)
}

fn error(witness: &Value) -> Option<String> {
    unfinished(&witness["observation"], &witness["clock"])
        .err()
        .map(|error| error.to_string())
}

fn synthetic_sleep_wait(witness: &mut Value) {
    future(
        &mut witness["observation"],
        json!({"FirstCompleted":[
            {"Single":{"CompletionId":2}},
            {"Single":{"SignalIndex":1}},
            {"Single":{"SignalName":"stop"}}
        ]}),
    );
}

#[test]
fn authentic_root06_refuses_sleep_without_awaited_completion() -> Result<()> {
    let witness = root06()?;
    assert_eq!(witness["proof"], Value::Null);
    assert_eq!(witness["parser_error"], Value::Null);
    assert_eq!(error(&witness), Some(MISSING_COMPLETION.to_string()));
    Ok(())
}

#[test]
fn synthetic_negative_signal_collisions_and_offsets_never_certify_sleep() -> Result<()> {
    for index in [0_u32, 1, 2, 3, u32::MAX] {
        let mut witness = root06()?;
        future(
            &mut witness["observation"],
            json!({"FirstCompleted":[
                {"Single":{"SignalIndex":index}},
                {"Single":{"SignalName":"stop"}}
            ]}),
        );
        assert_eq!(
            error(&witness),
            Some(MISSING_COMPLETION.to_string()),
            "{index}"
        );
    }
    Ok(())
}

#[test]
fn synthetic_negative_unrelated_completion_cannot_certify_abandoned_sleep() -> Result<()> {
    for completion in [0_u32, 1, 3, u32::MAX] {
        let mut witness = root06()?;
        future(
            &mut witness["observation"],
            json!({"FirstCompleted":[
                {"Single":{"CompletionId":completion}},
                {"Single":{"SignalIndex":2}},
                {"Single":{"SignalName":"stop"}}
            ]}),
        );
        assert_eq!(
            unfinished(&witness["observation"], &witness["clock"])?,
            None
        );
    }
    Ok(())
}

#[test]
fn synthetic_negative_matching_sleep_completion_disqualifies_boundary() -> Result<()> {
    let mut witness = root06()?;
    synthetic_sleep_wait(&mut witness);
    let clock = witness["clock"].clone();
    let observation = &mut witness["observation"];
    let id = observation["id"].clone();
    observation["journal"]["rows"]
        .as_array_mut()
        .context("Root06 journal absent")?
        .push(json!({"id":id,"index":4,"version":2,"entry_type":"Notification: Sleep",
            "entry_json":json!({"Notification":{"Completion":{"Sleep":{"completion_id":2}}}}).to_string()}));
    for side in ["before", "after"] {
        observation[side]["rows"][0]["journal_size"] = json!(5);
    }
    assert_eq!(unfinished(observation, &clock)?, None);
    Ok(())
}

#[test]
fn synthetic_negative_strict_wake_margin_refuses_exact_boundary() -> Result<()> {
    for realtime in ["2026-10-02T15:18:42.791Z", "2026-10-02T15:18:42.792Z"] {
        let mut witness = root06()?;
        synthetic_sleep_wait(&mut witness);
        witness["clock"]["realtime"] = json!(realtime);
        assert_eq!(
            unfinished(&witness["observation"], &witness["clock"])?,
            None
        );
    }
    Ok(())
}

#[test]
fn synthetic_negative_bookends_identity_and_full_journal_gates_remain_required() -> Result<()> {
    for field in [
        "modified_at",
        "pinned_deployment_id",
        "id",
        "target_service_key",
    ] {
        let mut witness = root06()?;
        synthetic_sleep_wait(&mut witness);
        witness["observation"]["after"]["rows"][0][field] = json!("foreign");
        assert_eq!(
            unfinished(&witness["observation"], &witness["clock"])?,
            None,
            "{field}"
        );
    }
    let mut witness = root06()?;
    synthetic_sleep_wait(&mut witness);
    witness["observation"]["journal"]["rows"]
        .as_array_mut()
        .context("Root06 journal absent")?
        .truncate(3);
    assert_eq!(
        unfinished(&witness["observation"], &witness["clock"])?,
        None
    );
    Ok(())
}

#[test]
fn synthetic_negative_malformed_evidence_is_not_hidden_by_matching_sleep() -> Result<()> {
    let mut witness = root06()?;
    synthetic_sleep_wait(&mut witness);
    witness["observation"]["journal"]["rows"][2]["index"] = json!(3);
    assert_eq!(
        error(&witness),
        Some("incomplete journal indexes".to_string())
    );
    let mut witness = root06()?;
    future(
        &mut witness["observation"],
        json!({"FirstCompleted":[
            {"Single":{"CompletionId":2}},
            {"Single":{"SignalIndex":4294967296_u64}}
        ]}),
    );
    assert_eq!(
        error(&witness),
        Some("notification index outside u32".to_string())
    );
    let mut witness = root06()?;
    synthetic_sleep_wait(&mut witness);
    witness["clock"]["realtime"] = Value::Null;
    assert_eq!(error(&witness), Some("guest clock absent".to_string()));
    Ok(())
}
