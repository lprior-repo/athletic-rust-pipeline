use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Cursor;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn prepared() -> Result<(Vec<u8>, String), serde_json::Error> {
    let bytes = serde_json::to_vec(&json!({
        "schema": "local-identity-adjudication-approval/v2",
        "actor": "local_operator",
        "prepared": {"version": 3, "operation": "split", "rationale": "Exact retained claims"},
        "graph": {"version": 1, "capture": "exact capture", "seal": "exact seal"}
    }))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok((bytes, digest))
}

#[test]
fn exact_confirmation_preserves_store_encoded_record_and_full_graph() -> TestResult {
    let (bytes, digest) = prepared()?;
    let mut input = Cursor::new(format!("approve {digest}\n"));
    let mut output = Vec::new();
    let approved = confirm(bytes.clone(), digest.clone(), &mut input, &mut output)?;
    let (actual, actual_digest) = approved.into_approved_record();
    check!(eq; actual, bytes);
    check!(eq; actual_digest, digest);
    let record: Value = serde_json::from_slice(&actual)?;
    check!(eq; record.get("schema"), Some(&json!("local-identity-adjudication-approval/v2")));
    check!(eq; record.get("actor"), Some(&json!("local_operator")));
    check!(eq; record.pointer("/graph/seal"), Some(&json!("exact seal")));
    check!(String::from_utf8(output)?.contains(&format!("approve {digest}")));
    Ok(())
}

#[test]
fn changed_store_approval_digest_requires_new_confirmation() -> TestResult {
    let (bytes, digest) = prepared()?;
    let mut input = Cursor::new(format!("approve {digest}\n"));
    check!(matches!(
        confirm(bytes, "a".repeat(64), &mut input, &mut Vec::new()),
        Err(ApprovalError::ExactConfirmationRequired)
    ));
    Ok(())
}

#[test]
fn piped_input_or_redirected_output_cannot_approve() -> TestResult {
    for (input, output) in [(false, false), (false, true), (true, false)] {
        check!(matches!(
            require_terminal(input, output),
            Err(ApprovalError::InteractiveTerminalRequired)
        ));
    }
    require_terminal(true, true)?;
    Ok(())
}

#[test]
fn generic_partial_and_unterminated_confirmations_are_rejected() -> TestResult {
    let (_, digest) = prepared()?;
    for response in [
        String::new(),
        "yes\n".into(),
        "approve\n".into(),
        format!("approve {digest}"),
        format!(" approve {digest}\n"),
        format!("approve {digest} \n"),
        format!("approve {digest}suffix\n"),
    ] {
        check!(matches!(
            verify_confirmation(&digest, response.as_bytes()),
            Err(ApprovalError::ExactConfirmationRequired)
        ));
    }
    verify_confirmation(&digest, format!("approve {digest}\r\n").as_bytes())?;
    Ok(())
}

#[test]
fn encoded_approval_budget_is_exact_and_refuses_before_prompting() -> TestResult {
    let digest = "a".repeat(64);
    let mut input = Cursor::new(format!("approve {digest}\n"));
    let approved = confirm(
        vec![b'x'; MAX_PREPARED_BYTES],
        digest.clone(),
        &mut input,
        &mut Vec::new(),
    )?;
    check!(eq; approved.into_approved_record().0.len(), MAX_PREPARED_BYTES);
    for bytes in [Vec::new(), vec![b'x'; MAX_PREPARED_BYTES + 1]] {
        let mut input = Cursor::new(b"yes\n");
        let mut output = Vec::new();
        check!(matches!(
            confirm(bytes, digest.clone(), &mut input, &mut output),
            Err(ApprovalError::PreparedBudgetExceeded)
        ));
        check!(output.is_empty());
        check!(eq; input.position(), 0);
    }
    Ok(())
}

#[test]
fn malformed_digest_refuses_before_prompting() -> TestResult {
    for digest in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
    ] {
        let mut input = Cursor::new(b"yes\n");
        let mut output = Vec::new();
        check!(matches!(
            confirm(b"record".to_vec(), digest, &mut input, &mut output),
            Err(ApprovalError::InvalidDigest)
        ));
        check!(output.is_empty());
        check!(eq; input.position(), 0);
    }
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
fn failed_prompt_never_consumes_confirmation() -> TestResult {
    let (bytes, digest) = prepared()?;
    let mut input = Cursor::new(b"approve whatever\n");
    check!(matches!(
        confirm(bytes, digest, &mut input, &mut RefusedOutput),
        Err(ApprovalError::Io(_))
    ));
    check!(eq; input.position(), 0);
    Ok(())
}

#[test]
fn overlong_confirmation_stops_at_fixed_bound() -> TestResult {
    let (bytes, digest) = prepared()?;
    let mut input = Cursor::new(vec![b'x'; 4096]);
    check!(matches!(
        confirm(bytes, digest, &mut input, &mut Vec::new()),
        Err(ApprovalError::ExactConfirmationRequired)
    ));
    check!(eq; input.position(), MAX_CONFIRMATION_BYTES);
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
fn failed_flush_never_consumes_confirmation() -> TestResult {
    let (bytes, digest) = prepared()?;
    let mut input = Cursor::new(b"approve whatever\n");
    check!(matches!(
        confirm(bytes, digest, &mut input, &mut FailedFlush(Vec::new())),
        Err(ApprovalError::Io(_))
    ));
    check!(eq; input.position(), 0);
    Ok(())
}
