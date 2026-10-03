use crate::workbook::ProjectedValue;
use calamine::DataRef;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    Empty,
    Text(String),
    Number(f64),
}

impl Value {
    pub(super) fn text(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Text(value.to_string())
        }
    }

    pub(super) fn optional(value: Option<impl AsRef<str>>) -> Self {
        value.map_or(Self::Empty, Self::text)
    }

    pub(super) fn integer(value: i64) -> Self {
        Self::Number(f64::from(
            i32::try_from(value).map_or(i32::MAX, |value| value),
        ))
    }

    pub(super) fn count(value: usize) -> Self {
        Self::Number(f64::from(
            u32::try_from(value).map_or(u32::MAX, |value| value),
        ))
    }

    pub(super) fn decimal(value: f64) -> Self {
        Self::Number(value)
    }

    pub(super) fn projected(projected: ProjectedValue<'_>) -> Self {
        match projected {
            ProjectedValue::Text(text) => Self::text(text),
            ProjectedValue::Number(Some(number)) => Self::Number(number),
            ProjectedValue::Number(None) => Self::Empty,
        }
    }

    pub(super) fn flagged(recorded: bool) -> Self {
        if recorded {
            Self::Text("yes".to_string())
        } else {
            Self::Empty
        }
    }

    pub(super) fn from_ref(cell: &DataRef<'_>) -> Self {
        match cell {
            DataRef::Empty => Self::Empty,
            DataRef::String(text) => Self::text(text),
            DataRef::SharedString(text) => Self::text(*text),
            DataRef::Float(number) => Self::Number(*number),
            DataRef::Int(number) => i32::try_from(*number).map_or_else(
                |_| Self::text(number.to_string()),
                |value| Self::Number(f64::from(value)),
            ),
            DataRef::Bool(value) => Self::text(value.to_string()),
            DataRef::DateTime(value) => Self::text(value.to_string()),
            DataRef::DateTimeIso(value) => Self::text(value),
            DataRef::DurationIso(value) => Self::text(value),
            DataRef::Error(error) => Self::text(error.to_string()),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("<blank>"),
            Self::Text(text) => write!(formatter, "{text:?}"),
            Self::Number(number) => write!(formatter, "{number}"),
        }
    }
}

pub(super) fn cell_at(sheet: &str, row: usize, column: usize) -> String {
    format!("{sheet}!{}", cell_reference(row, column))
}

pub(super) fn column_label(index: usize) -> String {
    let mut label = String::new();
    let mut value = index.saturating_add(1);
    while value > 0 {
        let remainder = value.saturating_sub(1) % 26;
        let letter = u8::try_from(remainder)
            .map_or(0, |value| value)
            .saturating_add(b'A');
        label.insert(0, char::from(letter));
        value = value.saturating_sub(1) / 26;
    }
    label
}

pub(super) fn cell_reference(row: usize, column: usize) -> String {
    format!("{}{}", column_label(column), row.saturating_add(1))
}
