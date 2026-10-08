use serde_json::Value;

use super::value_u64;
use census_domain::model::{CentiMetres, ExactSeconds};
use census_domain::model::{EventKind, Mark};

pub(super) fn row_mark(
    kind: &EventKind,
    published: Option<&str>,
    mark_int: Option<&Value>,
    validity: Option<&Value>,
) -> Option<Mark> {
    let published = published.map(str::trim).filter(|text| !text.is_empty());
    match source_status(published, validity) {
        SourceStatus::InvalidToken(token) => return Some(Mark::Raw(token.into())),
        SourceStatus::InvalidValidity(value) => {
            return Some(Mark::Raw(
                published.map_or_else(|| format!("vm={value}"), str::to_string),
            ))
        }
        SourceStatus::Valid => {}
    }
    if numeric_contradiction(kind, published, mark_int) {
        return published.map(|text| Mark::Raw(text.into()));
    }
    if let Some(text) = published {
        let Some(mark) = published_mark(kind, text) else {
            return Some(Mark::Raw(text.to_string()));
        };
        return Some(mark);
    }
    let integer = mark_int?;
    canonical_mark(kind, integer).or_else(|| Some(Mark::Raw(format!("im={integer}"))))
}

enum SourceStatus<'a, 'b> {
    Valid,
    InvalidToken(&'a str),
    InvalidValidity(&'b Value),
}

fn source_status<'a, 'b>(
    published: Option<&'a str>,
    validity: Option<&'b Value>,
) -> SourceStatus<'a, 'b> {
    if let Some(token) = published.and_then(crate::result_status::invalid_token) {
        return SourceStatus::InvalidToken(token);
    }
    match validity.filter(|value| !valid_mark(value)) {
        Some(value) => SourceStatus::InvalidValidity(value),
        None => SourceStatus::Valid,
    }
}

pub(super) fn mark_contradiction<'a>(
    kind: &EventKind,
    published: Option<&'a str>,
    mark_int: Option<&Value>,
    validity: Option<&Value>,
) -> Option<&'a str> {
    if numeric_contradiction(kind, published, mark_int) {
        return Some("numeric mark channels");
    }
    if value_u64(mark_int?)? == 0 {
        return None;
    }
    match source_status(published, validity) {
        SourceStatus::InvalidToken(token) => Some(token),
        SourceStatus::InvalidValidity(_) => Some("invalid vm"),
        SourceStatus::Valid => None,
    }
}

fn valid_mark(value: &Value) -> bool {
    match value {
        Value::Bool(valid) => *valid,
        value => value_u64(value) == Some(1),
    }
}

fn published_mark(kind: &EventKind, text: &str) -> Option<Mark> {
    if kind.is_field() {
        crate::hytek::parse_field_mark(text)
    } else {
        crate::hytek::parse_time(text).map(Mark::TimeSeconds)
    }
}
fn numeric_contradiction(
    kind: &EventKind,
    published: Option<&str>,
    integer: Option<&Value>,
) -> bool {
    let Some(display) = published.and_then(|text| published_mark(kind, text)) else {
        return false;
    };
    let Some(integer) = integer.filter(|value| !value.is_null()) else {
        return false;
    };
    let Some(measured) = canonical_mark(kind, integer) else {
        return true;
    };
    match (display, measured) {
        (Mark::TimeSeconds(display), Mark::TimeSeconds(measured)) => {
            let quantum = 9_u32
                .checked_sub(u32::from(display.precision()))
                .and_then(|exponent| 10_i64.checked_pow(exponent))
                .map_or(10_000_000, |value| value.max(10_000_000));
            display.value().abs_diff(measured.value())
                >= u64::try_from(quantum).map_or(u64::MAX, |value| value)
        }
        (
            Mark::FieldImperial {
                metres: display, ..
            }
            | Mark::DistanceMetres(display),
            Mark::DistanceMetres(measured),
        ) => display.value().abs_diff(measured.value()) > 1,
        _ => true,
    }
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
        let nanoseconds = i64::try_from(micros.checked_mul(1_000_000)?).ok()?;
        ExactSeconds::from_parts(nanoseconds, 3)
            .ok()
            .map(Mark::TimeSeconds)
    }
}
