use anyhow::{bail, Result};
use serde_json::Value;

const FORBIDDEN_KEYS: &[&str] = &[
    "email",
    "phone",
    "street",
    "postal",
    "cookie",
    "authorization",
    "token",
];

pub(crate) fn validate_value(value: &Value, path: &str) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let lower = key.to_ascii_lowercase();
                if FORBIDDEN_KEYS
                    .iter()
                    .any(|forbidden| lower.contains(forbidden))
                {
                    bail!("forbidden output field at {path}.{key}");
                }
                validate_value(child, &format!("{path}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                validate_value(child, &format!("{path}[{index}]"))?;
            }
        }
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            let approved_url = is_approved_url(text);
            let address_term = lower.split_whitespace().any(|word| {
                matches!(
                    word.trim_matches(|ch: char| !ch.is_ascii_alphabetic()),
                    "street" | "avenue" | "boulevard" | "road"
                )
            });
            if lower.contains("cookie")
                || lower.contains("authorization")
                || lower.contains("token")
                || (!approved_url && (address_term || has_postal_code(text)))
            {
                bail!("forbidden sensitive value at {path}");
            }
            if !approved_url && (looks_like_email(text) || looks_like_phone(text)) {
                bail!("forbidden personal value at {path}");
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn is_approved_url(text: &str) -> bool {
    crate::alpha_url::validate_profile_url(text).is_some()
        || crate::alpha_url::validate_result_url(text).is_some()
        || crate::alpha_url::validate_source_url(text).is_some()
}

fn has_postal_code(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_digit())
        .any(|part| part.len() == 5)
}

fn looks_like_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.is_empty() && !domain.is_empty() && !text.chars().any(char::is_whitespace)
}

fn looks_like_phone(text: &str) -> bool {
    let trimmed = text.trim();
    !is_iso_date(trimmed) && trimmed.chars().filter(|ch| ch.is_ascii_digit()).count() >= 7
}

fn is_iso_date(text: &str) -> bool {
    text.len() == 10
        && text.as_bytes().get(4) == Some(&b'-')
        && text.as_bytes().get(7) == Some(&b'-')
        && text
            .chars()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}
