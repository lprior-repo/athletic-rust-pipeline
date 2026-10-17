use crate::transport::transfer::refuse_excess_wire;
use crate::BrowserError;
use chromiumoxide::cdp::browser_protocol::network::RequestId;
use std::collections::HashMap;

const MAX_PENDING_TRANSFERS: usize = 64;
const MAX_BODY_BYTES: u32 = 33_554_432;

#[derive(Default)]
struct BodyProgress {
    encoded: u32,
    decoded: u32,
}

impl BodyProgress {
    fn observe(&mut self, encoded: i64, decoded: i64) -> Result<(), BrowserError> {
        let encoded = add_bytes(self.encoded, encoded)?;
        let decoded = add_bytes(self.decoded, decoded)?;
        self.encoded = encoded;
        self.decoded = decoded;
        Ok(())
    }
}

fn add_bytes(current: u32, incoming: i64) -> Result<u32, BrowserError> {
    if incoming < 0 {
        return Err(BrowserError::Protocol);
    }
    let incoming = u32::try_from(incoming).map_err(|_| BrowserError::PayloadLimit)?;
    current
        .checked_add(incoming)
        .filter(|total| *total <= MAX_BODY_BYTES)
        .ok_or(BrowserError::PayloadLimit)
}

#[derive(Default)]
pub(crate) struct WireTransfers {
    pending: HashMap<RequestId, BodyProgress>,
}

impl WireTransfers {
    pub(crate) fn bind(&mut self, id: &RequestId) {
        self.pending.retain(|known, _| known == id);
    }

    pub(crate) fn observe(
        &mut self,
        id: &RequestId,
        encoded: i64,
        decoded: i64,
    ) -> Result<(), BrowserError> {
        if !self.pending.contains_key(id) {
            if self.pending.len() >= MAX_PENDING_TRANSFERS {
                return Err(BrowserError::PayloadLimit);
            }
            self.pending
                .try_reserve(1)
                .map_err(|_| BrowserError::Unavailable)?;
        }
        self.pending
            .entry(id.clone())
            .or_default()
            .observe(encoded, decoded)
    }

    pub(crate) fn finish(&self, id: &RequestId, total: f64) -> Result<(), BrowserError> {
        refuse_excess_wire(Some(total))?;
        if self
            .pending
            .get(id)
            .is_some_and(|progress| total < f64::from(progress.encoded))
        {
            return Err(BrowserError::Protocol);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamed_encoded_and_decoded_caps_are_independent() {
        let mut encoded = BodyProgress::default();
        assert_eq!(encoded.observe(33_554_432, 1), Ok(()));
        assert_eq!(encoded.observe(1, 0), Err(BrowserError::PayloadLimit));
        let mut decoded = BodyProgress::default();
        assert_eq!(decoded.observe(1, 33_554_432), Ok(()));
        assert_eq!(decoded.observe(0, 1), Err(BrowserError::PayloadLimit));
    }

    #[test]
    fn invalid_chunks_never_wrap_or_partially_advance_counters() {
        let mut progress = BodyProgress::default();
        assert_eq!(progress.observe(7, 11), Ok(()));
        for (encoded, decoded, error) in [
            (-1, 0, BrowserError::Protocol),
            (0, -1, BrowserError::Protocol),
            (i64::MAX, 0, BrowserError::PayloadLimit),
            (0, i64::MAX, BrowserError::PayloadLimit),
            (1, 33_554_432, BrowserError::PayloadLimit),
        ] {
            assert_eq!(progress.observe(encoded, decoded), Err(error));
            assert_eq!((progress.encoded, progress.decoded), (7, 11));
        }
    }

    #[test]
    fn the_final_wire_total_cannot_erase_prebinding_observations() {
        let mut transfers = WireTransfers::default();
        let id = RequestId::new("request");
        assert_eq!(transfers.observe(&id, 4096, 8192), Ok(()));
        assert_eq!(transfers.finish(&id, 0.0), Err(BrowserError::Protocol));
        assert_eq!(transfers.finish(&id, 4096.0), Ok(()));
        assert_eq!(
            transfers.finish(&id, 33_554_433.0),
            Err(BrowserError::PayloadLimit)
        );
    }

    #[test]
    fn prebinding_observation_retention_is_bounded() {
        let mut transfers = WireTransfers::default();
        for index in 0..MAX_PENDING_TRANSFERS {
            assert_eq!(
                transfers.observe(&RequestId::new(index.to_string()), 1, 1),
                Ok(())
            );
        }
        assert_eq!(
            transfers.observe(&RequestId::new("excess"), 1, 1),
            Err(BrowserError::PayloadLimit)
        );
        assert_eq!(transfers.observe(&RequestId::new("0"), 1, 1), Ok(()));
        assert_eq!(
            transfers.finish(&RequestId::new("0"), 1.0),
            Err(BrowserError::Protocol)
        );
    }
}
