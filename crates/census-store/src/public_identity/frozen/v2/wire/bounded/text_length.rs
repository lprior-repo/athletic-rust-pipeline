use serde::de::Error;

pub(super) fn require<E: Error>(raw: &str, maximum: usize) -> Result<(), E> {
    let text = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| E::custom("frozen field is not a JSON string"))?;
    let mut bytes = text.bytes();
    std::iter::from_fn(|| bytes.next().map(|byte| width::<E>(byte, &mut bytes))).try_fold(
        0usize,
        |total, width| {
            let total = total
                .checked_add(width?)
                .ok_or_else(|| E::custom("frozen text overflow"))?;
            if total > maximum {
                return Err(E::custom("frozen decoded field ceiling"));
            }
            Ok(total)
        },
    )?;
    Ok(())
}
fn width<E: Error>(byte: u8, bytes: &mut std::str::Bytes<'_>) -> Result<usize, E> {
    if byte != b'\\' {
        if byte < 32 || byte == b'"' {
            return Err(E::custom("invalid frozen JSON string"));
        }
        return Ok(1);
    }
    match bytes.next() {
        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => Ok(1),
        Some(b'u') => unicode(bytes),
        _ => Err(E::custom("invalid frozen JSON escape")),
    }
}
fn unicode<E: Error>(bytes: &mut std::str::Bytes<'_>) -> Result<usize, E> {
    let value = hex::<E>(bytes)?;
    match value {
        0..=0x7f => Ok(1),
        0x80..=0x7ff => Ok(2),
        0xd800..=0xdbff => {
            if bytes.next() != Some(b'\\') || bytes.next() != Some(b'u') {
                return Err(E::custom("missing frozen JSON surrogate"));
            }
            if !(0xdc00..=0xdfff).contains(&hex::<E>(bytes)?) {
                return Err(E::custom("invalid frozen JSON surrogate"));
            }
            Ok(4)
        }
        0xdc00..=0xdfff => Err(E::custom("unpaired frozen JSON surrogate")),
        _ => Ok(3),
    }
}
fn hex<E: Error>(bytes: &mut std::str::Bytes<'_>) -> Result<u32, E> {
    (0..4).try_fold(0u32, |value, _| {
        let digit = bytes
            .next()
            .and_then(|byte| char::from(byte).to_digit(16))
            .ok_or_else(|| E::custom("invalid frozen JSON unicode escape"))?;
        value
            .checked_mul(16)
            .and_then(|value| value.checked_add(digit))
            .ok_or_else(|| E::custom("frozen JSON unicode overflow"))
    })
}
