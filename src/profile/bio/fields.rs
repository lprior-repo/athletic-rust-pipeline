//! Field helpers: bounded integers, optional text, evidence refs, and issue codes.

use serde_json::{Map, Value};

use crate::domain::evidence::{BestClaim, EvidenceIssue, EvidenceRef, Sport, SportAvailability};
use crate::domain::identity::EvidenceDigest;

use super::{MAX_ITEMS, MAX_SOURCE_ID};

pub(super) fn valid_short_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
pub(super) fn retain_cap_issue(
    length: usize,
    code: &str,
    locator: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) {
    if length > MAX_ITEMS {
        issues.push(issue(
            code,
            "source collection exceeded parser retention bound",
            Some(ev(digest, locator)),
        ));
    }
}
pub(super) fn best_claim(value: Option<&Value>, sport: Sport) -> BestClaim {
    match sport {
        Sport::TrackField => value
            .and_then(Value::as_u64)
            .map_or(BestClaim::Unavailable, BestClaim::OpaqueFlags),
        Sport::CrossCountry => {
            value
                .and_then(Value::as_bool)
                .map_or(BestClaim::Unavailable, |flag| {
                    if flag {
                        BestClaim::Claimed
                    } else {
                        BestClaim::NotClaimed
                    }
                })
        }
    }
}
pub(super) fn timing(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_u64).map(|item| {
        if item == 1 {
            "FAT".to_owned()
        } else if item == 0 {
            "hand".to_owned()
        } else {
            item.to_string()
        }
    })
}
pub(super) fn availability(sport: Sport, count: u64, present: bool) -> SportAvailability {
    if !present {
        SportAvailability::Unavailable { sport }
    } else if count == 0 {
        SportAvailability::EmptyResponse { sport }
    } else {
        SportAvailability::ResultsObserved { sport, count }
    }
}
pub(super) fn required_u64(
    obj: &Map<String, Value>,
    key: &str,
    locator: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> u64 {
    obj.get(key)
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value))
        .unwrap_or_else(|| {
            issues.push(issue(
                "invalid_result_id",
                &format!("result {key} is not a bounded positive integer"),
                Some(ev(digest, locator)),
            ));
            0
        })
}
pub(super) fn required_u16(
    obj: &Map<String, Value>,
    key: &str,
    locator: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> u16 {
    obj.get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or_else(|| {
            issues.push(issue(
                "invalid_season",
                "result SeasonID is invalid",
                Some(ev(digest, locator)),
            ));
            0
        })
}
pub(super) fn optional_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
pub(super) fn optional_display(value: Option<&Value>) -> Option<String> {
    value.filter(|item| !item.is_null()).map(display_value)
}
pub(super) fn display_value(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}
pub(super) fn ev(digest: &EvidenceDigest, locator: impl Into<String>) -> EvidenceRef {
    EvidenceRef {
        document: digest.clone(),
        locator: locator.into(),
    }
}
pub(super) fn bounded_id(value: u64) -> bool {
    value > 0 && value <= MAX_SOURCE_ID
}
pub(super) fn issue(code: &str, message: &str, evidence: Option<EvidenceRef>) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence,
    }
}
