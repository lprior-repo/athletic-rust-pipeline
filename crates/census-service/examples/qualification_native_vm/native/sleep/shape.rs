use anyhow::{ensure, Context, Result};
use serde_json::{Map, Value};
use std::borrow::Cow;

pub(super) fn decode(value: &Value) -> Result<Cow<'_, Value>> {
    match value {
        Value::String(raw) => Ok(Cow::Owned(serde_json::from_str(raw)?)),
        Value::Object(_) => Ok(Cow::Borrowed(value)),
        _ => anyhow::bail!("native journal JSON is neither object nor encoded JSON"),
    }
}

pub(super) fn variant<'a>(value: &'a Value, label: &str) -> Result<(&'a str, &'a Value)> {
    let object = value
        .as_object()
        .with_context(|| format!("{label} malformed"))?;
    ensure!(
        object.len() == 1,
        "{label} must contain exactly one variant"
    );
    object
        .iter()
        .next()
        .map(|(kind, payload)| (kind.as_str(), payload))
        .with_context(|| format!("{label} variant absent"))
}

pub(super) fn fields<'a>(
    value: &'a Value,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>> {
    let object = value.as_object().context("journal payload malformed")?;
    object.keys().try_for_each(|key| {
        ensure!(
            required.contains(&key.as_str()) || optional.contains(&key.as_str()),
            "unknown journal field: {key}"
        );
        Ok::<_, anyhow::Error>(())
    })?;
    required.iter().try_for_each(|key| {
        ensure!(
            object.contains_key(*key),
            "required journal field absent: {key}"
        );
        Ok::<_, anyhow::Error>(())
    })?;
    Ok(object)
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value> {
    value
        .get(key)
        .with_context(|| format!("required journal field absent: {key}"))
}

pub(super) fn text(value: &Value) -> Result<()> {
    ensure!(value.is_string(), "journal string malformed");
    Ok(())
}

pub(super) fn integer(value: &Value, maximum: u64, error: &str) -> Result<u64> {
    let number = value.as_u64().with_context(|| error.to_string())?;
    ensure!(number <= maximum, "{error}");
    Ok(number)
}

pub(super) fn completion(value: &Value) -> Result<u32> {
    let number = integer(
        field(value, "completion_id")?,
        u64::from(u32::MAX),
        "completion ID outside u32",
    )?;
    Ok(u32::try_from(number)?)
}

pub(super) fn bytes(value: &Value) -> Result<()> {
    let array = value.as_array().context("byte array malformed")?;
    ensure!(
        array.iter().all(|byte| byte
            .as_u64()
            .is_some_and(|number| number <= u64::from(u8::MAX))),
        "byte array malformed"
    );
    Ok(())
}

pub(super) fn strings(value: &Value) -> Result<()> {
    let array = value.as_array().context("string array malformed")?;
    array.iter().try_for_each(text)
}

pub(super) fn headers(value: &Value) -> Result<()> {
    value
        .as_array()
        .context("header array malformed")?
        .iter()
        .try_for_each(|header| {
            fields(header, &["name", "value"], &[])?;
            text(field(header, "name")?)?;
            text(field(header, "value")?)
        })
}

pub(super) fn result(value: &Value, allowed: &[&str]) -> Result<()> {
    if value.as_str() == Some("Void") {
        ensure!(allowed.contains(&"Void"), "result variant not allowed");
        return Ok(());
    }
    let (kind, payload) = variant(value, "result")?;
    ensure!(allowed.contains(&kind), "result variant not allowed");
    match kind {
        "Success" => bytes(payload),
        "Failure" => failure(payload),
        _ => anyhow::bail!("unknown result variant"),
    }
}

fn failure(value: &Value) -> Result<()> {
    fields(value, &["code", "message"], &["metadata"])?;
    integer(
        field(value, "code")?,
        u64::from(u16::MAX),
        "failure code outside u16",
    )?;
    text(field(value, "message")?)?;
    match value.get("metadata") {
        None => Ok(()),
        Some(metadata) => metadata
            .as_array()
            .context("failure metadata malformed")?
            .iter()
            .try_for_each(|item| {
                fields(item, &["key", "value"], &[])?;
                text(field(item, "key")?)?;
                text(field(item, "value")?)
            }),
    }
}

pub(super) fn signal(value: &Value) -> Result<()> {
    let (kind, payload) = variant(value, "signal ID")?;
    match kind {
        "Index" => integer(payload, u64::from(u32::MAX), "signal index outside u32").map(|_| ()),
        "Name" => text(payload),
        _ => anyhow::bail!("unknown signal ID variant"),
    }
}
