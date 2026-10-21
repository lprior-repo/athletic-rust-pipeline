use super::super::super::super::super::journal;
use super::{rows, text, Expected};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

pub(super) fn verify(status: &Value, retained: &Value, expected: &Expected) -> Result<Value> {
    ensure!(
        text(status, "status")? == "suspended",
        "original parent is not quiescent"
    );
    let id = text(status, "id")?;
    let entries =
        journal::complete(retained, status, id)?.context("child-wait journal incomplete")?;
    let calls = entries.iter().try_fold(Vec::new(), |mut found, row| -> Result<Vec<Value>> {
        let entry = journal::decode(row.get("entry_json").context("journal entry absent")?)?;
        let Some(call) = entry.pointer("/Command/Call") else { return Ok(found); };
        let request = call.get("request").context("child-wait call request absent")?;
        if text(request, "invocation_id")? != expected.child { return Ok(found); }
        validate_request(request, expected)?;
        ensure!(text(row, "entry_type")? == "Command: Call", "child-wait command type differs");
        ensure!(found.is_empty(), "duplicate exact child-wait call");
        found.try_reserve_exact(1)?;
        found.push(json!({"command":row,"invocation_completion":number(call,"invocation_id_completion_id")?,"result_completion":number(call,"result_completion_id")?}));
        Ok(found)
    })?;
    let [call] = calls.as_slice() else {
        anyhow::bail!("exact child call absent");
    };
    let result = number(call, "result_completion")?;
    let invocation = number(call, "invocation_completion")?;
    ensure!(result != invocation, "child completion identifiers overlap");
    let awaited = journal::awaited_child(status)?;
    ensure!(
        awaited.len() == 1 && awaited.contains(&result),
        "parent waits on something other than its exact child result"
    );
    ensure!(
        !journal::completions(entries)?.contains(&result),
        "child result already completed"
    );
    let acknowledgements = entries
        .iter()
        .try_fold(0usize, |count, row| -> Result<usize> {
            let entry = journal::decode(row.get("entry_json").context("journal entry absent")?)?;
            let Some(ack) = entry.pointer("/Notification/Completion/CallInvocationId") else {
                return Ok(count);
            };
            if number(ack, "completion_id")? != invocation {
                return Ok(count);
            }
            ensure!(
                text(ack, "invocation_id")? == expected.child,
                "child acknowledgement names another invocation"
            );
            count
                .checked_add(1)
                .context("child acknowledgement overflow")
        })?;
    ensure!(
        acknowledgements == 1,
        "exact child acknowledgement absent or duplicated"
    );
    Ok(
        json!({"child_id":expected.child,"child_service":expected.service,"child_key":expected.key,"result_completion":result,"invocation_completion":invocation,"command":call.get("command"),"journal_rows":rows(retained)?.len(),"proof":"quiescent exact unfinished child result; no self Sleep"}),
    )
}

fn validate_request(request: &Value, expected: &Expected) -> Result<()> {
    let target = request
        .pointer("/invocation_target/VirtualObject")
        .context("child target is not a virtual object")?;
    ensure!(
        text(target, "name")? == expected.service
            && text(target, "key")? == expected.key
            && text(target, "handler")? == "run"
            && text(target, "handler_ty")? == "Exclusive",
        "child-wait target differs"
    );
    let bytes = journal::bytes(request.get("parameter").context("child input absent")?)?;
    ensure!(
        serde_json::from_slice::<Value>(&bytes)? == expected.input,
        "child-wait full accepted input differs"
    );
    Ok(())
}

fn number(value: &Value, field: &str) -> Result<u32> {
    Ok(u32::try_from(
        value
            .get(field)
            .and_then(Value::as_u64)
            .with_context(|| format!("child-wait {field} absent"))?,
    )?)
}
