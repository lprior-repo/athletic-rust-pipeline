use serde_json::Value;

use super::value_u64;
use census_domain::model::{CentiMetres, CentiSeconds, EventKind, Mark};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MarkError {
    #[error("published no-result disposition contradicts a numeric mark or valid status")]
    NoResultConflict,
    #[error("invalid published result validity flag")]
    InvalidValidity,
    #[error("published display is not a supported result mark")]
    InvalidDisplay,
    #[error("published integer mark is outside the supported result range")]
    InvalidInteger,
    #[error("published display and integer mark disagree at provider centi-unit precision")]
    MarkConflict,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Disposition {
    Unstated,
    Valid,
    NoResult,
}

pub(super) fn row_mark(
    kind: &EventKind,
    published: Option<&str>,
    mark_int: Option<&Value>,
    validity: Option<&Value>,
) -> Result<Option<Mark>, MarkError> {
    let disposition = disposition(validity)?;
    let published = published.map(str::trim).filter(|text| !text.is_empty());
    let numeric = mark_int
        .map(|value| integer_mark(kind, value))
        .transpose()?
        .flatten();
    if published.is_some_and(no_result) || disposition == Disposition::NoResult {
        if numeric.is_some()
            || published
                .and_then(|text| display_mark(kind, text))
                .is_some()
            || (published.is_some_and(no_result) && disposition == Disposition::Valid)
        {
            return Err(MarkError::NoResultConflict);
        }
        return Ok(None);
    }
    let display = published
        .map(|text| display_mark(kind, text).ok_or(MarkError::InvalidDisplay))
        .transpose()?;
    reconcile(display, numeric, mark_int.is_some())
}

fn disposition(value: Option<&Value>) -> Result<Disposition, MarkError> {
    match value {
        None | Some(Value::Null) => Ok(Disposition::Unstated),
        Some(Value::Bool(true)) => Ok(Disposition::Valid),
        Some(Value::Bool(false)) => Ok(Disposition::NoResult),
        Some(value) => match value_u64(value) {
            Some(1) => Ok(Disposition::Valid),
            Some(0) => Ok(Disposition::NoResult),
            _ => Err(MarkError::InvalidValidity),
        },
    }
}

fn no_result(text: &str) -> bool {
    ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR", "--"]
        .iter()
        .any(|status| text.eq_ignore_ascii_case(status))
}

fn display_mark(kind: &EventKind, text: &str) -> Option<Mark> {
    if kind.is_field() {
        crate::hytek::parse_field_mark(text)
    } else {
        crate::hytek::parse_time(text).map(Mark::TimeSeconds)
    }
}

fn reconcile(
    display: Option<Mark>,
    numeric: Option<Mark>,
    integer_published: bool,
) -> Result<Option<Mark>, MarkError> {
    match (display, numeric) {
        (Some(display), Some(numeric)) if comparable(&display) != comparable(&numeric) => {
            Err(MarkError::MarkConflict)
        }
        (Some(_), None) if integer_published => Err(MarkError::MarkConflict),
        (Some(display), _) => Ok(Some(display)),
        (None, numeric) => Ok(numeric),
    }
}

fn comparable(mark: &Mark) -> Option<i32> {
    match mark {
        Mark::TimeSeconds(value) => Some(value.value()),
        Mark::DistanceMetres(value) | Mark::FieldImperial { metres: value, .. } => {
            Some(value.value())
        }
        _ => None,
    }
}

fn integer_mark(kind: &EventKind, value: &Value) -> Result<Option<Mark>, MarkError> {
    if value_u64(value) == Some(0) {
        return Ok(None);
    }
    canonical_mark(kind, value)
        .map(Some)
        .ok_or(MarkError::InvalidInteger)
}

pub(super) fn canonical_mark(kind: &EventKind, mark_int: &Value) -> Option<Mark> {
    let micros = value_u64(mark_int)?;
    if micros == 0 {
        return None;
    }
    if kind.is_field() {
        let cm = (micros.checked_add(5_000)? / 10_000).try_into().ok()?;
        Some(Mark::DistanceMetres(CentiMetres::new(cm)))
    } else {
        let cs = (micros.checked_add(5)? / 10).try_into().ok()?;
        Some(Mark::TimeSeconds(CentiSeconds::new(cs)))
    }
}
