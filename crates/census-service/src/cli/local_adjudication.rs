use std::io::{BufRead, IsTerminal, Read, Write};

mod command;
mod input;
mod membership;
pub(super) use command::{run, LocalAdjudicationArgs};

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
    #[error("prepared identity adjudication has an invalid approval digest")]
    InvalidDigest,
    #[error("identity adjudication terminal I/O failed: {0}")]
    Io(#[from] std::io::Error),
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

pub(super) fn approve_prepared(
    encoded: Vec<u8>,
    digest: String,
) -> Result<VerifiedLocalAdjudication, ApprovalError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    require_terminal(stdin.is_terminal(), stdout.is_terminal())?;
    confirm(encoded, digest, &mut stdin.lock(), &mut stdout.lock())
}

fn require_terminal(input: bool, output: bool) -> Result<(), ApprovalError> {
    match (input, output) {
        (true, true) => Ok(()),
        _ => Err(ApprovalError::InteractiveTerminalRequired),
    }
}

fn confirm<R: BufRead, W: Write>(
    encoded: Vec<u8>,
    digest: String,
    reader: &mut R,
    writer: &mut W,
) -> Result<VerifiedLocalAdjudication, ApprovalError> {
    if encoded.is_empty() || encoded.len() > MAX_PREPARED_BYTES {
        return Err(ApprovalError::PreparedBudgetExceeded);
    }
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(ApprovalError::InvalidDigest);
    }
    writer.write_all(
        b"Local operator identity adjudication. Review the complete prepared record and graph:\n",
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

#[cfg(test)]
mod tests;
