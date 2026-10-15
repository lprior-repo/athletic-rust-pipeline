use super::{
    invalid, raw, records::Records, reserve, wire, DigestV2, RawPhysicalRowV2, TableBindingV2,
    RAW_FRAGMENT_BYTES,
};
use crate::{StoreError, StoreResult};

pub(super) fn row(
    records: &mut Records<'_, '_>,
    binding: &TableBindingV2,
    index: u16,
    remaining: u32,
) -> StoreResult<(RawPhysicalRowV2, u32)> {
    let key = format!("row:{index:020}");
    let header: wire::RawRow = records.next(&key, "raw_row")?;
    let physical_key = raw::decode_key(&header.physical_key_base64)?;
    let length = usize::try_from(header.value_bytes).map_err(|_| StoreError::CounterOverflow)?;
    raw::check_lengths(physical_key.len(), length)?;
    let charged = u32::try_from(physical_key.len())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(header.value_bytes)
        .ok_or(StoreError::CounterOverflow)?;
    if charged > remaining {
        return Err(invalid("raw page allocation exceeds declared byte total"));
    }
    let parts = length.div_ceil(RAW_FRAGMENT_BYTES);
    if usize::from(header.parts) != parts {
        return Err(invalid("raw fragment count"));
    }
    let mut value = reserve(length)?;
    (0..header.parts).try_for_each(|part| {
        let payload: wire::RawPart = records.next(&format!("{key}:part:{part:020}"), "raw_part")?;
        let decoded = raw::decode_base64(&payload.bytes_base64, RAW_FRAGMENT_BYTES)?;
        let remaining = length
            .checked_sub(value.len())
            .ok_or(StoreError::CounterOverflow)?;
        if decoded.len() != remaining.min(RAW_FRAGMENT_BYTES) {
            return Err(invalid("raw fragment length"));
        }
        value.extend_from_slice(&decoded);
        Ok::<(), StoreError>(())
    })?;
    if value.len() != length || DigestV2::of_bytes(&value) != header.value_sha256 {
        return Err(invalid("raw value length or digest"));
    }
    Ok((
        RawPhysicalRowV2::from_parts(binding, physical_key, value)?,
        charged,
    ))
}
