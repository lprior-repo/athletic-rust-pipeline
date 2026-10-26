use std::io::{self, Write};

use super::super::script::FETCH_FUNCTION;
use super::super::FetchArguments;
use crate::BrowserError;

const MAX_EXPRESSION_BYTES: usize = 32 * 1024 * 1024;
const PREFIX: &[u8] = b"(function(){return (";
const BETWEEN: &[u8] = b")(";
const SUFFIX: &[u8] = b");})";

pub(super) fn build(arguments: &FetchArguments<'_>) -> Result<String, BrowserError> {
    let fixed = FETCH_FUNCTION
        .len()
        .checked_add(PREFIX.len())
        .and_then(|total| total.checked_add(BETWEEN.len()))
        .and_then(|total| total.checked_add(SUFFIX.len()))
        .ok_or(BrowserError::PayloadLimit)?;
    let available = MAX_EXPRESSION_BYTES
        .checked_sub(fixed)
        .ok_or(BrowserError::PayloadLimit)?;
    let mut measure = Measure {
        bytes: 0,
        maximum: available,
        refused: false,
    };
    let measured = serde_json::to_writer(&mut measure, arguments);
    if measure.refused {
        return Err(BrowserError::PayloadLimit);
    }
    measured.map_err(|_| BrowserError::Protocol)?;
    let total = fixed
        .checked_add(measure.bytes)
        .ok_or(BrowserError::PayloadLimit)?;
    let mut output = Expression {
        bytes: Vec::new(),
        maximum: total,
        refused: false,
    };
    output
        .bytes
        .try_reserve_exact(total)
        .map_err(|_| BrowserError::PayloadLimit)?;
    output
        .write_all(PREFIX)
        .map_err(|_| BrowserError::PayloadLimit)?;
    output
        .write_all(FETCH_FUNCTION.as_bytes())
        .map_err(|_| BrowserError::PayloadLimit)?;
    output
        .write_all(BETWEEN)
        .map_err(|_| BrowserError::PayloadLimit)?;
    let encoded = serde_json::to_writer(&mut output, arguments);
    if output.refused {
        return Err(BrowserError::PayloadLimit);
    }
    encoded.map_err(|_| BrowserError::Protocol)?;
    output
        .write_all(SUFFIX)
        .map_err(|_| BrowserError::PayloadLimit)?;
    if output.bytes.len() != total {
        return Err(BrowserError::Protocol);
    }
    String::from_utf8(output.bytes).map_err(|_| BrowserError::Protocol)
}

struct Measure {
    bytes: usize,
    maximum: usize,
    refused: bool,
}

impl Write for Measure {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.bytes.checked_add(bytes.len()) {
            Some(total) if total <= self.maximum => {
                self.bytes = total;
                Ok(bytes.len())
            }
            _ => {
                self.refused = true;
                Err(io::ErrorKind::WriteZero.into())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Expression {
    bytes: Vec<u8>,
    maximum: usize,
    refused: bool,
}

impl Write for Expression {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.bytes.len().checked_add(bytes.len()) {
            Some(total) if total <= self.maximum => {
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            _ => {
                self.refused = true;
                Err(io::ErrorKind::WriteZero.into())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{redirect_mode, FetchArguments};
    use super::build;

    fn arguments(redirect: &'static str) -> FetchArguments<'static> {
        FetchArguments {
            url: "https://www.milesplit.com/athletes/1-a",
            method: "GET",
            body: None,
            timeout_ms: 1_000,
            max_body: 8,
            headers: &[],
            redirect,
        }
    }

    #[test]
    fn the_page_refuses_redirects_unless_following_is_enabled() {
        let refusing = build(&arguments(redirect_mode(false))).expect("expression builds");
        assert!(
            refusing.contains(r#""redirect":"error""#),
            "default deployments keep redirect:error, got: {refusing}"
        );
        assert!(!refusing.contains(r#""redirect":"follow""#));
    }

    #[test]
    fn opted_in_deployments_let_the_page_follow_redirects() {
        let following = build(&arguments(redirect_mode(true))).expect("expression builds");
        assert!(
            following.contains(r#""redirect":"follow""#),
            "opt-in deployments send redirect:follow, got: {following}"
        );
        assert!(!following.contains(r#""redirect":"error""#));
    }
}
