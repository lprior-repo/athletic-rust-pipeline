use super::{call, Expected};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

fn entry(index: u64, kind: &str, value: Value) -> Value {
    json!({"id":"inv_parent","index":index,"version":2,"entry_type":kind,"entry_json":value})
}

fn fixture() -> Result<(Value, Value, Expected)> {
    let input = json!({"jurisdiction":"RI","season":2026,"revision":1,"refresh":false});
    let expected = Expected {
        child: "inv_child".to_owned(),
        service: "JurisdictionCensus",
        key: "jurisdiction:RI:2026-27:1".to_owned(),
        input: input.clone(),
    };
    let retained = json!({"rows":[
        entry(0,"Command: Input",json!({"Command":{"Input":{"payload":[]}}})),
        entry(1,"Command: Call",json!({"Command":{"Call":{"request":{"invocation_id":"inv_child","invocation_target":{"VirtualObject":{"name":expected.service,"key":expected.key,"handler":"run","handler_ty":"Exclusive"}},"parameter":serde_json::to_vec(&input)?},"invocation_id_completion_id":5,"result_completion_id":6}}})),
        entry(2,"Notification: CallInvocationId",json!({"Notification":{"Completion":{"CallInvocationId":{"completion_id":5,"invocation_id":"inv_child"}}}}))]});
    Ok((
        json!({"id":"inv_parent","status":"suspended","journal_size":3,"suspended_waiting_future_json":{"Single":{"CompletionId":6}}}),
        retained,
        expected,
    ))
}

fn replace(value: &mut Value, path: &str, new: Value) -> Result<()> {
    *value.pointer_mut(path).context("fixture path missing")? = new;
    Ok(())
}

#[test]
fn exact_unfinished_original_child_result_qualifies_quiescent_parent() -> Result<()> {
    let (status, retained, expected) = fixture()?;
    let proof = call::verify(&status, &retained, &expected)?;
    ensure!(
        proof.get("child_id") == Some(&json!("inv_child"))
            && proof.get("result_completion") == Some(&json!(6)),
        "wrong child proof"
    );
    Ok(())
}

#[test]
fn sleep_other_child_signal_duplicate_or_completed_future_refuses() -> Result<()> {
    let (status, retained, expected) = fixture()?;
    for future in [
        json!({"Single":{"CompletionId":99}}),
        json!({"Single":{"SignalName":"stop"}}),
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"CompletionId":6}}]}),
    ] {
        let mut changed = status.clone();
        replace(&mut changed, "/suspended_waiting_future_json", future)?;
        ensure!(
            call::verify(&changed, &retained, &expected).is_err(),
            "non-child-only future accepted"
        );
    }
    let mut completed = retained.clone();
    completed.get_mut("rows").and_then(Value::as_array_mut).context("rows absent")?.push(entry(3,"Notification: Call",json!({"Notification":{"Completion":{"Call":{"completion_id":6,"result":{"Success":[]}}}}})));
    let mut updated = status.clone();
    replace(&mut updated, "/journal_size", json!(4))?;
    ensure!(
        call::verify(&updated, &completed, &expected).is_err(),
        "completed child treated as unfinished"
    );
    let mut sleep = retained.clone();
    replace(
        &mut sleep,
        "/rows/1/entry_json",
        json!({"Command":{"Sleep":{"completion_id":6,"wake_up_time":1}}}),
    )?;
    ensure!(
        call::verify(&status, &sleep, &expected).is_err(),
        "self Sleep qualified as child wait"
    );
    Ok(())
}

#[test]
fn changed_full_input_target_ack_or_truncated_journal_refuses() -> Result<()> {
    let (status, retained, expected) = fixture()?;
    for (path, value) in [
        (
            "/rows/1/entry_json/Command/Call/request/parameter",
            json!(serde_json::to_vec(
                &json!({"jurisdiction":"RI","season":2026,"revision":1,"refresh":true})
            )?),
        ),
        (
            "/rows/1/entry_json/Command/Call/request/invocation_target/VirtualObject/key",
            json!("jurisdiction:MI:2026-27:1"),
        ),
        (
            "/rows/2/entry_json/Notification/Completion/CallInvocationId/invocation_id",
            json!("inv_foreign"),
        ),
    ] {
        let mut changed = retained.clone();
        replace(&mut changed, path, value)?;
        ensure!(
            call::verify(&status, &changed, &expected).is_err(),
            "changed accepted request/identity qualified"
        );
    }
    let mut truncated = retained.clone();
    truncated
        .get_mut("rows")
        .and_then(Value::as_array_mut)
        .context("rows absent")?
        .pop();
    ensure!(
        call::verify(&status, &truncated, &expected).is_err(),
        "incomplete prefix qualified"
    );
    let mut running = status;
    replace(&mut running, "/status", json!("running"))?;
    ensure!(
        call::verify(&running, &retained, &expected).is_err(),
        "running parent qualified"
    );
    Ok(())
}

#[test]
fn sdk_cancellation_envelope_preserves_exact_child_proof_and_original_bytes() -> Result<()> {
    let (status, retained, expected) = fixture()?;
    let original = call::verify(&status, &retained, &expected)?;
    let retained_bytes = serde_json::to_vec(&retained)?;
    for future in [
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":1}}]}),
        json!({"FirstCompleted":[{"Single":{"SignalIndex":1}},{"Single":{"CompletionId":6}}]}),
    ] {
        for value in [
            future.clone(),
            Value::String(serde_json::to_string(&future)?),
        ] {
            let mut changed = status.clone();
            replace(&mut changed, "/suspended_waiting_future_json", value)?;
            let status_bytes = serde_json::to_vec(&changed)?;
            ensure!(
                call::verify(&changed, &retained, &expected)? == original,
                "SDK cancellation envelope changed exact child proof"
            );
            ensure!(
                serde_json::to_vec(&changed)? == status_bytes
                    && serde_json::to_vec(&retained)? == retained_bytes,
                "future oracle changed retained status or journal bytes"
            );
        }
    }
    Ok(())
}

#[test]
fn only_one_exact_outer_sdk_cancellation_envelope_is_accepted() -> Result<()> {
    let (status, retained, expected) = fixture()?;
    for future in [
        json!({"Single":{"SignalIndex":1}}),
        json!({"FirstCompleted":[{"Single":{"SignalIndex":1}},{"Single":{"SignalIndex":1}}]}),
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":2}}]}),
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalName":"cancel"}}]}),
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":1}},{"Single":{"CompletionId":99}}]}),
        json!({"AllCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":1}}]}),
        json!({"FirstCompleted":[{"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":1}}]},{"Single":{"SignalIndex":1}}]}),
        json!({"AllCompleted":[{"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":1}}]}]}),
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":1,"SignalName":"cancel"}}]}),
        json!({"FirstCompleted":[{"Single":{"CompletionId":6}},{"Single":{"SignalIndex":"1"}}]}),
    ] {
        let mut changed = status.clone();
        replace(&mut changed, "/suspended_waiting_future_json", future)?;
        ensure!(
            call::verify(&changed, &retained, &expected).is_err(),
            "malformed or misplaced cancellation envelope accepted"
        );
    }
    Ok(())
}
