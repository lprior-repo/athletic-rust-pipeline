
use census_domain::model::VerdictBatch;
use serde_json::Value;

use super::ModelError;

pub(super) fn message_content(body: &Value) -> Option<String> {
    let message = body.get("choices")?.get(0)?.get("message")?;
    let content = content_text(message)?;
    let trimmed = content.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

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

pub fn parse_batch(content: &str) -> Result<VerdictBatch, ModelError> {
    let cleaned = strip_fence(content);
    if let Ok(batch) = serde_json::from_str::<VerdictBatch>(cleaned) {
        return Ok(batch);
    }
    if let Ok(verdicts) = serde_json::from_str::<Vec<census_domain::model::ReviewVerdict>>(cleaned) {
        return Ok(VerdictBatch {
            subject_id: String::new(),
            verdicts,
        });
    }
    Err(ModelError::Content {
        reason: "not a verdict batch",
    })
}

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
