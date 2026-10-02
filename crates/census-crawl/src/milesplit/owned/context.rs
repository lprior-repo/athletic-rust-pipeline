use super::OwnedRejectionKind;
use serde_json::Value;

type ContextResult = Result<(), (OwnedRejectionKind, String)>;

pub(super) fn validate_context(row: &Value) -> ContextResult {
    [
        "teamName",
        "round",
        "roundName",
        "divisionName",
        "eventCode",
    ]
    .into_iter()
    .try_for_each(|field| check(row, field, |value| value.is_string()))?;
    [
        "heat",
        "units",
        "eventDistance",
        "divisionId",
        "meetResultsDivisionId",
        "resultsDivisionId",
    ]
    .into_iter()
    .try_for_each(|field| check(row, field, |value| unsigned(value).is_some()))?;
    check(row, "place", |value| {
        unsigned(value).is_some_and(|place| u16::try_from(place).is_ok())
    })?;
    check(row, "windReading", |value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|token| token.parse::<f64>().ok()))
            .is_some_and(f64::is_finite)
    })
}

fn unsigned(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value.as_str().and_then(|token| {
            token
                .bytes()
                .all(|byte| byte.is_ascii_digit())
                .then(|| token.parse::<u64>().ok())
                .flatten()
        })
    })
}

fn check(row: &Value, field: &str, valid: impl FnOnce(&Value) -> bool) -> ContextResult {
    let Some(value) = row.get(field).filter(|value| !value.is_null()) else {
        return Ok(());
    };
    if valid(value) {
        return Ok(());
    }
    Err((
        OwnedRejectionKind::InvalidContext,
        format!("malformed published {field}"),
    ))
}
