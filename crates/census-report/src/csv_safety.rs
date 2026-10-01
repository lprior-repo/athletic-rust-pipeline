use serde::{ser::Error, Serialize, Serializer};
use std::borrow::Cow;
#[derive(Debug, thiserror::Error)]
enum CsvTextError {
    #[error("CSV text capacity exhausted")]
    Capacity,
    #[error("allocating literal CSV text: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
}

fn executable(value: &str) -> bool {
    value
        .chars()
        .find(|character| {
            !character.is_whitespace() && !character.is_control() && *character != '\u{feff}'
        })
        .is_some_and(|character| matches!(character, '=' | '+' | '-' | '@'))
}

pub(crate) fn protect_owned(
    mut value: String,
) -> Result<String, std::collections::TryReserveError> {
    if executable(&value) {
        value.try_reserve(1)?;
        value.insert(0, '\'');
    }
    Ok(value)
}

fn protect(value: &str) -> Result<Cow<'_, str>, CsvTextError> {
    if !executable(value) {
        return Ok(Cow::Borrowed(value));
    }
    let mut escaped = String::new();
    let capacity = value.len().checked_add(1).ok_or(CsvTextError::Capacity)?;
    escaped.try_reserve_exact(capacity)?;
    escaped.push('\'');
    escaped.push_str(value);
    Ok(Cow::Owned(escaped))
}

pub(crate) fn serialize_text<S: Serializer>(value: &str, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&protect(value).map_err(S::Error::custom)?)
}

pub(crate) fn serialize_optional_text<S: Serializer>(
    value: &Option<&str>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.serialize_some(&Protected(value)),
        None => serializer.serialize_none(),
    }
}

struct Protected<'a>(&'a str);
impl Serialize for Protected<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_text(self.0, serializer)
    }
}
