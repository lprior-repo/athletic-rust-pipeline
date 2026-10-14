use serde_json::Value;

pub(super) fn count_at(parsed: &Value, field: &str) -> Option<usize> {
    usize::try_from(parsed.get(field)?.as_u64()?).ok()
}

pub(super) fn length(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}
