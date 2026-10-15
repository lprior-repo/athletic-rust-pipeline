use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{BufRead, IsTerminal, Read, Write};

mod command;
mod input;
pub(super) use command::{run, LocalAdjudicationArgs};

const APPROVAL_SCHEMA: &str = "local-identity-adjudication-approval/v1";
const MAX_PREPARED_BYTES: usize = 1024 * 1024;
const MAX_CONFIRMATION_BYTES: u64 = 75;

#[derive(Debug, thiserror::Error)]
pub(super) enum ApprovalError {
    #[error("identity adjudication requires an interactive local terminal")]
    InteractiveTerminalRequired,
    #[error("identity adjudication was not confirmed for the exact prepared digest")]
    ExactConfirmationRequired,
    #[error("prepared identity adjudication exceeds the encoded approval budget")]
    PreparedBudgetExceeded,
    #[error("identity adjudication preparation could not be encoded: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("identity adjudication terminal I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Actor {
    LocalOperator,
}

#[derive(Serialize)]
struct ApprovalEnvelope<'a, T> {
    schema: &'static str,
    actor: Actor,
    prepared: &'a T,
}

pub(super) struct VerifiedLocalAdjudication {
    encoded: Vec<u8>,
    digest: String,
}

impl VerifiedLocalAdjudication {
    pub(super) fn into_approved_record(self) -> (Vec<u8>, String) {
        (self.encoded, self.digest)
    }
}

pub(super) fn approve_prepared<T: Serialize>(
    prepared: &T,
) -> Result<VerifiedLocalAdjudication, ApprovalError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    require_terminal(stdin.is_terminal(), stdout.is_terminal())?;
    confirm(prepared, &mut stdin.lock(), &mut stdout.lock())
}

fn require_terminal(input: bool, output: bool) -> Result<(), ApprovalError> {
    match (input, output) {
        (true, true) => Ok(()),
        _ => Err(ApprovalError::InteractiveTerminalRequired),
    }
}

fn confirm<T: Serialize, R: BufRead, W: Write>(
    prepared: &T,
    reader: &mut R,
    writer: &mut W,
) -> Result<VerifiedLocalAdjudication, ApprovalError> {
    let encoded = encode(prepared)?;
    let digest = approval_digest(&encoded);
    writer.write_all(
        b"Local operator identity adjudication. Review the complete prepared record:\n",
    )?;
    writer.write_all(&encoded)?;
    write!(writer, "\nType exactly: approve {digest}\n> ")?;
    writer.flush()?;
    let mut response = Vec::new();
    response
        .try_reserve_exact(75)
        .map_err(std::io::Error::other)?;
    reader
        .take(MAX_CONFIRMATION_BYTES)
        .read_until(b'\n', &mut response)?;
    verify_confirmation(&digest, &response)?;
    Ok(VerifiedLocalAdjudication { encoded, digest })
}

fn verify_confirmation(digest: &str, response: &[u8]) -> Result<(), ApprovalError> {
    let expected = format!("approve {digest}");
    let line = response
        .strip_suffix(b"\r\n")
        .or_else(|| response.strip_suffix(b"\n"));
    if line != Some(expected.as_bytes()) {
        return Err(ApprovalError::ExactConfirmationRequired);
    }
    Ok(())
}

fn approval_digest(encoded: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(APPROVAL_SCHEMA.as_bytes());
    digest.update([0]);
    digest.update(encoded);
    format!("{:x}", digest.finalize())
}

fn encode<T: Serialize>(prepared: &T) -> Result<Vec<u8>, ApprovalError> {
    let envelope = ApprovalEnvelope {
        schema: APPROVAL_SCHEMA,
        actor: Actor::LocalOperator,
        prepared,
    };
    let mut writer = BoundedRecord {
        bytes: Vec::new(),
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, &envelope);
    if writer.exceeded {
        return Err(ApprovalError::PreparedBudgetExceeded);
    }
    result.map_err(ApprovalError::Encode)?;
    Ok(writer.bytes)
}

struct BoundedRecord {
    bytes: Vec<u8>,
    exceeded: bool,
}

impl Write for BoundedRecord {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let remaining = MAX_PREPARED_BYTES.saturating_sub(self.bytes.len());
        if bytes.len() > remaining {
            self.exceeded = true;
            return Err(std::io::Error::other("prepared adjudication budget"));
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(std::io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
