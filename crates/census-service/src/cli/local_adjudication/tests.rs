use super::*;
use serde_json::{json, Value};
use std::io::Cursor;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn prepared() -> Value {
    json!({
        "operation": "split", "case_id": "synthetic-case", "revision": 8,
        "expected": {"store": "synthetic-store", "head": "head-7", "context_digest": "context-7"},
        "supersedes": [{"decision": "merge-6", "digest": "merge-digest"}],
        "members": [{"candidate": "a", "digest": "a-evidence"}, {"candidate": "b", "digest": "b-evidence"}],
        "citations": [{"capture": "capture-1", "interpretation": "parser-2", "locator": "row-3", "claim_digest": "claim-4"}],
        "conflict_dispositions": [{"claim": "old-link", "resolution": "rejected_for_identity", "citation": 0}],
        "partitions": [["a"], ["b"]], "unresolved_records": ["result-uncertain"],
        "rationale": "Synthetic contradictory identity claim resolved using cited retained context",
        "policy": "local-reviewed-identity/v1"
    })
}

#[test]
fn exact_confirmation_freezes_the_complete_record_and_local_operator_provenance() -> TestResult {
    let prepared = prepared();
    let encoded = encode(&prepared)?;
    let digest = approval_digest(&encoded);
    let mut input = Cursor::new(format!("approve {digest}\n"));
    let mut output = Vec::new();
    let approved = confirm(&prepared, &mut input, &mut output)?;
    let (actual, actual_digest) = approved.into_approved_record();
    assert_eq!(actual, encoded);
    assert_eq!(actual_digest, digest);
    let record: Value = serde_json::from_slice(&actual)?;
    assert_eq!(record.get("actor"), Some(&json!("local_operator")));
    assert_eq!(record.get("prepared"), Some(&prepared));
    assert!(String::from_utf8(output)?.contains(&format!("approve {digest}")));
    Ok(())
}

#[test]
fn a_changed_operation_state_citation_disposition_or_result_needs_new_confirmation() -> TestResult {
    let original = prepared();
    let digest = approval_digest(&encode(&original)?);
    for field in [
        "operation",
        "case_id",
        "revision",
        "expected",
        "supersedes",
        "members",
        "citations",
        "conflict_dispositions",
        "partitions",
        "unresolved_records",
        "rationale",
        "policy",
    ] {
        let mut changed = original.clone();
        let object = changed.as_object_mut().ok_or("fixture is not an object")?;
        object.insert(field.to_owned(), json!("changed"));
        let mut input = Cursor::new(format!("approve {digest}\n"));
        let result = confirm(&changed, &mut input, &mut Vec::new());
        assert!(
            matches!(result, Err(ApprovalError::ExactConfirmationRequired)),
            "{field}"
        );
    }
    Ok(())
}

#[test]
fn piped_input_or_redirected_output_cannot_approve() {
    for (input, output) in [(false, false), (false, true), (true, false)] {
        assert!(matches!(
            require_terminal(input, output),
            Err(ApprovalError::InteractiveTerminalRequired)
        ));
    }
    assert!(require_terminal(true, true).is_ok());
}

#[test]
fn generic_consent_partial_confirmation_and_unterminated_input_are_rejected() -> TestResult {
    let digest = approval_digest(&encode(&prepared())?);
    for response in [
        String::new(),
        "yes\n".to_owned(),
        "approve\n".to_owned(),
        format!("approve {digest}"),
        format!(" approve {digest}\n"),
        format!("approve {digest} \n"),
        format!("approve {digest}suffix\n"),
    ] {
        assert!(matches!(
            verify_confirmation(&digest, response.as_bytes()),
            Err(ApprovalError::ExactConfirmationRequired)
        ));
    }
    verify_confirmation(&digest, format!("approve {digest}\r\n").as_bytes())?;
    Ok(())
}

#[test]
fn oversized_prepared_record_is_refused_before_prompting() -> TestResult {
    let mut input = Cursor::new(b"yes\n");
    let mut output = Vec::new();
    let result = confirm(&"x".repeat(MAX_PREPARED_BYTES), &mut input, &mut output);
    assert!(matches!(result, Err(ApprovalError::PreparedBudgetExceeded)));
    assert!(output.is_empty());
    assert_eq!(input.position(), 0);
    Ok(())
}

#[test]
fn exact_encoded_budget_is_accepted_and_one_more_byte_is_refused() -> TestResult {
    let overhead = encode(&"")?.len();
    let payload = "x".repeat(MAX_PREPARED_BYTES.checked_sub(overhead).ok_or("budget")?);
    assert_eq!(encode(&payload)?.len(), MAX_PREPARED_BYTES);
    assert!(matches!(
        encode(&format!("{payload}x")),
        Err(ApprovalError::PreparedBudgetExceeded)
    ));
    Ok(())
}

struct RefusedOutput;
impl Write for RefusedOutput {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("synthetic output failure"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn failed_prompt_never_consumes_confirmation() {
    let mut input = Cursor::new(b"approve whatever\n");
    let result = confirm(&prepared(), &mut input, &mut RefusedOutput);
    assert!(matches!(result, Err(ApprovalError::Io(_))));
    assert_eq!(input.position(), 0);
}

#[test]
fn overlong_confirmation_stops_reading_at_the_fixed_bound() -> TestResult {
    let mut input = Cursor::new(vec![b'x'; 4096]);
    let result = confirm(&prepared(), &mut input, &mut Vec::new());
    assert!(matches!(
        result,
        Err(ApprovalError::ExactConfirmationRequired)
    ));
    assert_eq!(input.position(), MAX_CONFIRMATION_BYTES);
    Ok(())
}

struct FailedFlush(Vec<u8>);
impl Write for FailedFlush {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("synthetic flush failure"))
    }
}

#[test]
fn a_prompt_that_cannot_be_flushed_does_not_consume_confirmation() {
    let mut input = Cursor::new(b"approve whatever\n");
    let result = confirm(&prepared(), &mut input, &mut FailedFlush(Vec::new()));
    assert!(matches!(result, Err(ApprovalError::Io(_))));
    assert_eq!(input.position(), 0);
}
