use super::*;

pub(super) fn legacy(raw: &[u8]) -> StoreResult<Rows> {
    let physical = Store::journal_key(PHASE, KEY);
    if raw.len() <= MAX_JOURNAL_VALUE_BYTES {
        return Ok(vec![(physical, raw.to_vec())]);
    }
    let total = raw.len().div_ceil(crate::journal::JOURNAL_CHUNK_RAW);
    let mut rows = vec![(
        physical.clone(),
        crate::journal::manifest_value(KEY, total, raw.len())?,
    )];
    for (index, part) in raw.chunks(crate::journal::JOURNAL_CHUNK_RAW).enumerate() {
        rows.push((
            crate::journal::chunk_key(&physical, index)?,
            crate::journal::chunk_value(KEY, index, total, part)?,
        ));
    }
    Ok(rows)
}

pub(super) fn expected_cost(raw: &[u8]) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let key = Store::journal_key(PHASE, KEY).len();
    if raw.len() <= MAX_JOURNAL_VALUE_BYTES {
        return Ok((raw.len().checked_add(key).ok_or("bytes")?, 3));
    }
    let mut bytes = 500_000usize
        .checked_add(512)
        .and_then(|n| n.checked_add(key.checked_mul(2)?))
        .ok_or("buffer and manifest")?;
    let mut steps = 4usize;
    for part in raw.chunks(500_000) {
        let encoded = part.len().checked_add(2).ok_or("round")? / 3;
        bytes = bytes
            .checked_add(encoded.checked_mul(4).ok_or("base64")?)
            .and_then(|n| n.checked_add(512))
            .and_then(|n| n.checked_add(key))
            .and_then(|n| n.checked_add(5))
            .ok_or("chunk")?;
        steps = steps.checked_add(5).ok_or("steps")?;
    }
    Ok((bytes, steps))
}
