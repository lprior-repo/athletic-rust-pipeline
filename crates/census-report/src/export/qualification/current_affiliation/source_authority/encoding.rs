use super::*;
use crate::export::proof_objects::MAX_OBJECT_BYTES;
use std::io::Write;

pub(super) fn encode(
    value: &impl serde::Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let ceiling = usize::try_from(MAX_OBJECT_BYTES).map_err(|_| StoreError::CounterOverflow)?;
    let expected = measure(value, ceiling, grant)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected)
        .map_err(|_| invalid("roster family inventory allocation failed"))?;
    census_domain::model::write_serialized(&mut bytes, value)?;
    if bytes.len() != expected {
        return Err(invalid("roster family inventory changed during encoding"));
    }
    Ok(bytes)
}

pub(super) fn family(
    common: &impl serde::Serialize,
    withdrawals: &[census_store::ReviewedRosterMemberWithdrawalV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<SourceAuthorityFamilyInventoryV1> {
    let bytes = if withdrawals.is_empty() {
        encode(common, grant)?
    } else {
        encode(
            &("roster-source-family-inventory-v3", common, withdrawals),
            grant,
        )?
    };
    let digest = hash(&bytes);
    Ok(SourceAuthorityFamilyInventoryV1 { bytes, digest })
}

struct Counter {
    bytes: usize,
    ceiling: usize,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("roster inventory arithmetic overflow"))?;
        if count > self.ceiling {
            return Err(std::io::Error::other(
                "roster inventory byte ceiling exceeded",
            ));
        }
        self.bytes = count;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn admit(
    value: &impl serde::Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<usize> {
    measure(value, 8 * 1024 * 1024, grant)
}

fn measure(
    value: &impl serde::Serialize,
    ceiling: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<usize> {
    let mut counter = Counter { bytes: 0, ceiling };
    census_domain::model::write_serialized(&mut counter, value)?;
    charge(grant, counter.bytes, 1)?;
    Ok(counter.bytes)
}

#[cfg(test)]
mod tests;
