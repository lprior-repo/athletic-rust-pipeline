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
    let batch: VerdictBatch = serde_json::from_str(content).map_err(|_| ModelError::Content {
        reason: "not a verdict batch",
    })?;
    if batch.verdicts.iter().any(|verdict| {
        verdict.field.is_none() || verdict.value.is_none() || verdict.confidence > 100
    }) {
        return Err(ModelError::Content {
            reason: "verdict fields must be strings and confidence must be at most 100",
        });
    }
    Ok(batch)
}
