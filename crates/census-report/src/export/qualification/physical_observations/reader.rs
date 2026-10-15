use super::*;

pub(super) fn visit<T: Entity>(
    objects_dir: &Path,
    descriptor: &PhysicalObservationDescriptor,
    budget: ReadBudget,
    mut visit: impl FnMut(T) -> StoreResult<()>,
) -> ReportResult<ReadCounts> {
    let inventory: Inventory = object::decode(objects_dir, &descriptor.inventory)?;
    if inventory.version != 1
        || inventory.table != descriptor.table
        || inventory.chunks.len() > MAX_CHUNKS
        || inventory.rows != descriptor.rows
        || inventory.bytes != descriptor.bytes
        || inventory.digest != descriptor.fingerprint
    {
        return Err(
            invalid("frozen physical observation inventory differs from its descriptor").into(),
        );
    }
    let mut closure = Closure::new(descriptor.table, budget)?;
    for reference in &inventory.chunks {
        let bytes = object::read(
            objects_dir,
            reference,
            u64::try_from(CHUNK_BYTES).map_err(|_| StoreError::CounterOverflow)?,
        )?;
        if bytes.is_empty() {
            return Err(invalid("frozen physical observation chunk is empty").into());
        }
        let mut cursor = bytes.as_slice();
        while !cursor.is_empty() {
            let header = take(&mut cursor, 8)?;
            let key_length = length(
                header
                    .get(..4)
                    .ok_or_else(|| invalid("missing physical key length"))?,
            )?;
            let raw_length = length(
                header
                    .get(4..)
                    .ok_or_else(|| invalid("missing physical row length"))?,
            )?;
            if key_length > census_store::MAX_ID_BYTES + 64 || raw_length > RECORD_BYTES {
                return Err(invalid("frozen physical row framing exceeds bounded lengths").into());
            }
            let key = take(&mut cursor, key_length)?;
            let raw = take(&mut cursor, raw_length)?;
            closure.push(key, raw)?;
            let row = census_store::decode_physical_observation::<T>(descriptor.table, key, raw)?;
            visit(row)?;
        }
    }
    let (counts, digest) = closure.finish();
    if counts != descriptor.counts() || digest != inventory.digest {
        return Err(invalid(
            "frozen physical observation closure does not reproduce its pinned table fingerprint",
        )
        .into());
    }
    Ok(counts)
}

fn length(bytes: &[u8]) -> StoreResult<usize> {
    let raw: [u8; 4] = bytes
        .try_into()
        .map_err(|_| invalid("frozen physical row length is truncated"))?;
    usize::try_from(u32::from_be_bytes(raw)).map_err(|_| StoreError::CounterOverflow)
}

fn take<'a>(cursor: &mut &'a [u8], length: usize) -> StoreResult<&'a [u8]> {
    let original = *cursor;
    let value = original
        .get(..length)
        .ok_or_else(|| invalid("frozen physical row frame is truncated"))?;
    *cursor = original
        .get(length..)
        .ok_or_else(|| invalid("frozen physical row cursor overflow"))?;
    Ok(value)
}
