use super::budget::check_json_budget;
use super::codec::{
    check_entity_id, corrupt, count, encoding_error, FrozenCodecError, FrozenCodecResult,
    LINE_BYTES_LIMIT, MAX_JSON_NODES,
};
use census_domain::model::write_serialized;
use census_store::Entity;
use serde::{Deserialize, Serialize};
use std::io;

const ORDINAL_PREFIX: &[u8] = b"{\"ordinal\":";

pub struct DecodedLine<T> {
    pub ordinal: u64,
    pub entity_id: String,
    pub payload: T,
}

#[derive(Serialize)]
struct LineRef<'a, T: ?Sized> {
    ordinal: u64,
    entity_id: &'a str,
    payload: &'a T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineOwned<T> {
    ordinal: u64,
    entity_id: String,
    payload: T,
}

pub fn write_line<W: io::Write, T: Entity>(
    writer: &mut W,
    ordinal: u64,
    entity_id: &str,
    payload: &T,
) -> FrozenCodecResult<()> {
    write_serialized(
        writer,
        &LineRef {
            ordinal,
            entity_id,
            payload,
        },
    )
    .map_err(encoding_error)
}

#[cfg(test)]
pub fn line_bytes<T: Entity>(
    ordinal: u64,
    entity_id: &str,
    payload: &T,
) -> FrozenCodecResult<Vec<u8>> {
    let mut line = Vec::new();
    write_line(&mut line, ordinal, entity_id, payload)?;
    check_line_total(count(line.len())?)?;
    Ok(line)
}

#[cfg(test)]
pub fn encode_line<T: Entity>(ordinal: u64, payload: &T) -> FrozenCodecResult<Vec<u8>> {
    let entity_id = payload.entity_id();
    check_entity_id(entity_id)?;
    line_bytes(ordinal, entity_id, payload)
}

pub fn decode_line<T: Entity>(line: &[u8]) -> FrozenCodecResult<DecodedLine<T>> {
    check_line_total(count(line.len())?)?;
    if !line.starts_with(ORDINAL_PREFIX) {
        return Err(corrupt(
            "frozen line",
            "a line must begin with the pinned {\"ordinal\": field",
        ));
    }
    check_json_budget("frozen line", line, MAX_JSON_NODES)?;
    let parsed: LineOwned<T> = serde_json::from_slice(line)
        .map_err(|source| corrupt("frozen line", &source.to_string()))?;
    check_entity_id(&parsed.entity_id)?;
    let entity_id = parsed.entity_id;
    let payload_id = parsed.payload.entity_id();
    if payload_id != entity_id.as_str() {
        return Err(corrupt(
            "frozen line",
            &format!("payload id {payload_id} does not match entity_id {entity_id}"),
        ));
    }
    Ok(DecodedLine {
        ordinal: parsed.ordinal,
        entity_id,
        payload: parsed.payload,
    })
}

fn check_line_total(bytes: u64) -> FrozenCodecResult<()> {
    let total = bytes
        .checked_add(1)
        .ok_or_else(|| corrupt("frozen line", "a line length overflowed u64"))?;
    if total > LINE_BYTES_LIMIT {
        return Err(FrozenCodecError::RowTooLarge {
            what: "frozen line including its newline",
            bytes: total,
            max: LINE_BYTES_LIMIT,
        });
    }
    Ok(())
}
