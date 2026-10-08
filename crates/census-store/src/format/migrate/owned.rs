use serde_json::Value;

use super::exact::{convert_mark, decode, encode, invalid};
use crate::{Store, StoreResult};

pub(super) const ARCHIVE_PHASE: &str = "schema1_lineage_milesplit_owned_meet_v1";
const PHASES: [&str; 3] = [
    "milesplit_owned_meet_v1",
    "milesplit_owned_meet_v2",
    "milesplit_owned_meet_v3",
];

pub(super) struct Rekeyed {
    pub(super) key: Vec<u8>,
    pub(super) archive: Vec<u8>,
    pub(super) value: Vec<u8>,
}

pub(super) fn convert(key: &[u8], bytes: &[u8]) -> StoreResult<Option<Rekeyed>> {
    let Some((phase, locator)) = owned_locator(key)? else {
        return Ok(None);
    };
    let mut entry = decode(bytes)?;
    let archive = Store::journal_key(ARCHIVE_PHASE, &format!("original/{phase}/{locator}"));
    if entry.get("key").and_then(Value::as_str) != Some(locator) {
        return Err(invalid(
            "owned journal envelope key does not match its durable key",
        ));
    }
    let payload = entry
        .get_mut("payload")
        .ok_or_else(|| invalid("owned row lacks payload"))?;
    let base = row_base(locator, payload)?;
    let mark = payload
        .get_mut("mark")
        .ok_or_else(|| invalid("owned row lacks canonical mark"))?;
    if !convert_mark(mark)? {
        return Ok(None);
    }
    let locator = format!("{base}/{}", digest(payload)?);
    entry
        .as_object_mut()
        .ok_or_else(|| invalid("owned journal envelope is not an object"))?
        .insert("key".into(), Value::String(locator.clone()));
    Ok(Some(Rekeyed {
        archive,
        key: Store::journal_key(phase, &locator),
        value: encode(&entry)?,
    }))
}

fn owned_locator(key: &[u8]) -> StoreResult<Option<(&str, &str)>> {
    let Some(separator) = key.iter().position(|byte| *byte == 0) else {
        return Ok(None);
    };
    let phase = std::str::from_utf8(
        key.get(..separator)
            .ok_or_else(|| invalid("journal phase"))?,
    )
    .map_err(|_| invalid("non-utf8 journal phase"))?;
    if !PHASES.contains(&phase) {
        return Ok(None);
    }
    let start = separator
        .checked_add(1)
        .ok_or(crate::StoreError::CounterOverflow)?;
    let locator = std::str::from_utf8(key.get(start..).ok_or_else(|| invalid("journal locator"))?)
        .map_err(|_| invalid("non-utf8 owned journal locator"))?;
    Ok(locator.starts_with("row/").then_some((phase, locator)))
}

fn row_base<'a>(locator: &'a str, payload: &Value) -> StoreResult<&'a str> {
    let Some((base, suffix)) = locator.rsplit_once('/') else {
        return Err(invalid("owned row locator"));
    };
    if suffix.len() != 64 || !suffix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(locator);
    }
    if suffix != digest(payload)? {
        return Err(invalid("owned row digest does not match payload"));
    }
    Ok(base)
}

fn digest(payload: &Value) -> StoreResult<String> {
    census_domain::model::serialized_digest(payload)
        .map_err(|error| invalid(&format!("hashing migrated owned canonical row: {error}")))
}
