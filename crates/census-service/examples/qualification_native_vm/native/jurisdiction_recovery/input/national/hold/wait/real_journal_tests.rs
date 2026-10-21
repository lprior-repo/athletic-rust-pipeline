use super::super::super::super::super::journal;
use super::{call, Expected};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::JurisdictionRequest;
use serde_json::{json, Value};

#[test]
fn controlled_suspended_projection_of_actual_native165_journal_binds_original_child_six(
) -> Result<()> {
    let retained: Value =
        serde_json::from_str(include_str!("tests/native165-parent-journal.json"))?;
    let command = retained
        .pointer("/rows/9/entry_json")
        .context("actual call absent")?;
    let decoded = journal::decode(command)?;
    let request = decoded
        .pointer("/Command/Call/request")
        .context("actual child request absent")?;
    let child = request
        .get("invocation_id")
        .and_then(Value::as_str)
        .context("actual child ID absent")?;
    let input: JurisdictionRequest = serde_json::from_slice(&journal::bytes(
        request
            .get("parameter")
            .context("actual child input absent")?,
    )?)?;
    let expected = Expected::national(child, &input)?;
    let status = json!({"id":"inv_17BPDiBMH80F0FwaHPcM3VleI2Ql6hrse6","status":"suspended","journal_size":11,"suspended_waiting_future_json":"{\"Single\":{\"CompletionId\":6}}"});
    let proof = call::verify(&status, &retained, &expected)?;
    ensure!(
        proof.get("child_id") == Some(&json!("inv_1j4peqPCiaI62lqSOEvZ41N2ChV6NRwdLX"))
            && proof.get("result_completion") == Some(&json!(6)),
        "actual original child changed"
    );
    let mut different_input = input;
    different_input.refresh = true;
    ensure!(
        call::verify(
            &status,
            &retained,
            &Expected::national(child, &different_input)?
        )
        .is_err(),
        "different accepted request qualified"
    );
    let mut self_sleep = status;
    *self_sleep
        .get_mut("suspended_waiting_future_json")
        .context("future absent")? = json!({"Single":{"CompletionId":2}});
    ensure!(
        call::verify(&self_sleep, &retained, &expected).is_err(),
        "unrelated original completion qualified"
    );
    Ok(())
}
