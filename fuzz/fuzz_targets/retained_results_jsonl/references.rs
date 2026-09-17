use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

// Native audit captures contain fresh UUIDs. Bind only the resulting digest
// references; all other committed synthetic fixture values must stay unchanged.
pub(super) fn collect(
    before: &Value,
    after: &Value,
    bindings: &mut BTreeMap<String, String>,
) -> Result<()> {
    if before == after {
        return Ok(());
    }
    match (before, after) {
        (Value::String(old), Value::String(new)) if digest(old) && digest(new) => {
            if bindings
                .insert(old.clone(), new.clone())
                .is_some_and(|existing| existing != *new)
            {
                bail!("fixture digest {old} has conflicting capture bindings");
            }
        }
        (Value::Array(old), Value::Array(new)) if old.len() == new.len() => {
            old.iter()
                .zip(new)
                .try_for_each(|(old, new)| collect(old, new, bindings))?;
        }
        (Value::Object(old), Value::Object(new)) if old.len() == new.len() => {
            old.iter().try_for_each(|(key, old)| {
                collect(
                    old,
                    new.get(key).context("fixture field disappeared")?,
                    bindings,
                )
            })?;
        }
        _ => bail!("fixture hydration changed more than audit digest references"),
    }
    Ok(())
}

fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn bind(data: &[u8], bindings: &BTreeMap<String, String>) -> Vec<u8> {
    let mut rewritten = data.to_vec();
    // Equal-length byte replacement preserves malformed JSON, duplicate keys,
    // whitespace and mutations. Parsing/reserializing would hide parser bugs.
    for (offset, token) in data.windows(66).enumerate() {
        if token.first() != Some(&b'"') || token.last() != Some(&b'"') {
            continue;
        }
        let Some(body) = token.get(1..65) else {
            continue;
        };
        let Some(replacement) = std::str::from_utf8(body)
            .ok()
            .and_then(|value| bindings.get(value))
        else {
            continue;
        };
        if let Some(destination) = rewritten.get_mut(offset + 1..offset + 65) {
            destination.copy_from_slice(replacement.as_bytes());
        }
    }
    rewritten
}
