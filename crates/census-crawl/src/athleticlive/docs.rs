use serde_json::Value;

mod events;
mod marks;
mod rows;

pub(super) use events::SummaryEvent;
pub use events::{parse_event_document, parse_event_summary, EventDoc};
pub use rows::{DocRow, DocTeam};

pub(super) fn value_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

pub(super) fn value_flag(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.as_i64().is_some_and(|v| v != 0),
        Value::Bool(b) => *b,
        Value::String(s) => s.trim() != "0" && !s.trim().is_empty(),
        _ => false,
    }
}
