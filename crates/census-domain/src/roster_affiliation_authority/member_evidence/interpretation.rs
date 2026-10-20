use super::{Result, SourceRowLocator};
use crate::roster_affiliation_authority::RosterAuthorityError;
use serde_json::Value;

pub(super) struct Row<'a> {
    pub(super) locator: SourceRowLocator,
    pub(super) provider_id: &'a str,
    pub(super) semantic_digest: &'a str,
}

pub(super) fn decode(bytes: &[u8]) -> Result<Value> {
    let value: Value = serde_json::from_slice(bytes)?;
    if text(&value, "/schema")? != "roster-retained-capture-interpretation-v3" {
        return Err(RosterAuthorityError::Invalid("interpretation schema"));
    }
    if text(&value, "/semantic_inventory/schema")? != "roster-source-semantic-inventory-v2" {
        return Err(RosterAuthorityError::Invalid("inventory schema"));
    }
    if text(&value, "/semantic_inventory/policy")? != "milesplit-row-local-current-roster/v2" {
        return Err(RosterAuthorityError::Invalid("inventory policy"));
    }
    Ok(value)
}

pub(super) fn row_at(value: &Value, index: usize) -> Result<Row<'_>> {
    let rows = value
        .pointer("/semantic_inventory/rows")
        .and_then(Value::as_array)
        .ok_or(RosterAuthorityError::Invalid("inventory rows"))?;
    if rows.is_empty() || rows.len() > 256 {
        return Err(RosterAuthorityError::Invalid("interpretation row count"));
    }
    let row = rows.get(index).ok_or(RosterAuthorityError::Invalid(
        "witness row index out of range",
    ))?;
    if text(row, "/outcome/state")? != "admitted" {
        return Err(RosterAuthorityError::Invalid("witness row not admitted"));
    }
    Ok(Row {
        locator: locator(
            row.get("row")
                .ok_or(RosterAuthorityError::Invalid("row missing locator"))?,
        )?,
        provider_id: text(row, "/provider_id")?,
        semantic_digest: text(row, "/outcome/semantic_digest")?,
    })
}

fn text<'a>(value: &'a Value, path: &str) -> Result<&'a str> {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .ok_or(RosterAuthorityError::Invalid("inventory text field"))
}

fn locator(value: &Value) -> Result<SourceRowLocator> {
    let ordinal =
        u32::try_from(number(value, "ordinal")?).map_err(|_| RosterAuthorityError::Overflow)?;
    let offset = usize::try_from(number(value, "byte_offset")?)
        .map_err(|_| RosterAuthorityError::Overflow)?;
    let length = usize::try_from(number(value, "byte_length")?)
        .map_err(|_| RosterAuthorityError::Overflow)?;
    Ok(SourceRowLocator::new(ordinal, offset, length))
}

fn number(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(RosterAuthorityError::Invalid("locator integer field"))
}
