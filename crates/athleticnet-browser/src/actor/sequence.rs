use crate::BrowserError;

const MAX_JAVASCRIPT_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Default)]
pub(super) struct CaptureSequence(u64);

impl CaptureSequence {
    pub(super) fn issue(&mut self) -> Result<u64, BrowserError> {
        let nonce = self.0;
        if nonce > MAX_JAVASCRIPT_INTEGER {
            return Err(BrowserError::Protocol);
        }
        self.0 = nonce.checked_add(1).ok_or(BrowserError::Protocol)?;
        Ok(nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_nonces_do_not_round_or_wrap_at_the_javascript_boundary() {
        let mut sequence = CaptureSequence(MAX_JAVASCRIPT_INTEGER);
        assert_eq!(sequence.issue(), Ok(9_007_199_254_740_991));
        assert_eq!(sequence.issue(), Err(BrowserError::Protocol));
        assert_eq!(sequence.issue(), Err(BrowserError::Protocol));
        assert_eq!(
            CaptureSequence(u64::MAX).issue(),
            Err(BrowserError::Protocol)
        );
    }
}
