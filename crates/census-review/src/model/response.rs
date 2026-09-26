//! The response half: reading the one field that carries content, and the batch that survives
//! sanitizing.

use census_domain::model::VerdictBatch;
use serde_json::Value;

use super::ModelError;

pub(super) fn message_content(body: &Value) -> Option<String> {
    let message = body.get("choices")?.get(0)?.get("message")?;
    let content = content_text(message)?;
    let trimmed = content.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Content as text: a string, or the text parts of a content array.
pub(super) fn content_text(message: &Value) -> Option<String> {
    match message.get("content")? {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let joined: String = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            Some(joined)
        }
        _ => None,
    }
}

/// Read a verdict batch out of the model's content.
///
/// The grammar should make this a formality, but models and servers both drift: a fenced block is
/// stripped, and a bare array of verdicts is accepted with the batch's subject left unstated — the
/// caller matches verdicts to the packet it sent, so a missing subject is not a reason to discard an
/// answer.
pub fn parse_batch(content: &str) -> Result<VerdictBatch, ModelError> {
    let cleaned = strip_fence(content);
    if let Ok(batch) = serde_json::from_str::<VerdictBatch>(cleaned) {
        return Ok(batch);
    }
    if let Ok(verdicts) = serde_json::from_str::<Vec<census_domain::model::ReviewVerdict>>(cleaned)
    {
        return Ok(VerdictBatch {
            subject_id: String::new(),
            verdicts,
        });
    }
    match serde_json::from_str::<VerdictBatch>(cleaned) {
        Ok(batch) => Ok(batch),
        Err(source) => Err(ModelError::Content {
            content: truncate(cleaned, 400),
            source,
        }),
    }
}

/// Remove a Markdown code fence, which some servers add around otherwise valid JSON.
pub(super) fn strip_fence(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest
        .strip_prefix("json")
        .or_else(|| rest.strip_prefix("JSON"))
        .unwrap_or(rest);
    rest.trim_start()
        .strip_suffix("```")
        .map(str::trim_end)
        .unwrap_or_else(|| rest.trim_end())
}

/// A bounded copy of a string for error messages.
pub(super) fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    match text.get(..end) {
        Some(head) => format!("{head}…"),
        None => text.to_string(),
    }
}
