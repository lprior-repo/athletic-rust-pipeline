use super::budget::account_row_bytes;
use anyhow::{bail, Result};
use calamine::DataRef;

pub(super) fn materialize_cell_text(value: &DataRef<'_>, row_bytes: &mut usize) -> Result<String> {
    match value {
        DataRef::String(text) => {
            account_row_bytes(row_bytes, text.len())?;
            Ok(text.clone())
        }
        DataRef::SharedString(text) => {
            account_row_bytes(row_bytes, text.len())?;
            Ok((*text).to_owned())
        }
        DataRef::DateTimeIso(text) | DataRef::DurationIso(text) => {
            account_row_bytes(row_bytes, text.len())?;
            Ok(text.clone())
        }
        other => {
            let text = cell_text(other)?;
            account_row_bytes(row_bytes, text.len())?;
            Ok(text)
        }
    }
}

fn cell_text(value: &DataRef<'_>) -> Result<String> {
    match value {
        DataRef::Int(number) => Ok(number.to_string()),
        DataRef::Float(number) => finite_number(*number),
        DataRef::String(text) => Ok(text.clone()),
        DataRef::SharedString(text) => Ok((*text).to_owned()),
        DataRef::Bool(value) => Ok(if *value {
            "1".to_owned()
        } else {
            "0".to_owned()
        }),
        DataRef::DateTime(value) => finite_number(value.as_f64()),
        DataRef::DateTimeIso(text) => Ok(text.clone()),
        DataRef::DurationIso(text) => Ok(text.clone()),
        DataRef::Error(error) => Ok(error.to_string()),
        DataRef::Empty => Ok(String::new()),
    }
}

fn finite_number(number: f64) -> Result<String> {
    if !number.is_finite() {
        bail!("nonfinite numeric cell value cannot be represented");
    }
    Ok(number.to_string())
}
