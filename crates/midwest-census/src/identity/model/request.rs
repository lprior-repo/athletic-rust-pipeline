//! The request half: the body the client sends, built from one packet and the options.

use census_domain::model::ReviewPacket;
use serde_json::{json, Map, Value};

use super::ModelOptions;

pub fn build_request_body(packet: &ReviewPacket, options: &ModelOptions) -> Value {
    let mut body = Map::new();
    body.insert("model".to_string(), json!(options.model));
    body.insert(
        "messages".to_string(),
        json!([
            { "role": "system", "content": system_prompt() },
            { "role": "user", "content": render_packet(packet) },
        ]),
    );
    body.insert("temperature".to_string(), json!(0));
    body.insert("max_tokens".to_string(), json!(options.max_tokens));
    body.insert(
        "response_format".to_string(),
        json!({
            "type": "json_schema",
            "json_schema": {
                "name": "review_verdicts",
                "strict": true,
                "schema": verdict_schema(),
            }
        }),
    );
    // A reasoning model's thinking is spent from the same output budget as its answer, and this
    // task's answer is short: ask the template not to think.
    body.insert(
        "chat_template_kwargs".to_string(),
        json!({ "enable_thinking": false }),
    );
    Value::Object(body)
}

/// The rule the model must follow, stated once per request.
pub(super) fn system_prompt() -> String {
    [
        "You adjudicate retained census findings about schools, meets and athletes.",
        "Each case names a field that no stored source resolved.",
        "",
        "Answer with one verdict per case, in the order given:",
        "- If the subject's own evidence settles the field, answer value_proposed and put the field",
        "  name and the value in `field` and `value`.",
        "- Otherwise answer insufficient_evidence and leave `field` and `value` empty strings.",
        "",
        "Rules:",
        "- Use only the evidence given; never invent a school, a state or a venue that is not implied",
        "  by it.",
        "- The value must be the shortest form the field asks for: a two-letter state code for `state`,",
        "  a two-letter state code for `state`.",
        "- `confidence` is 0-100 and reports how certain the evidence makes you, not how helpful the",
        "  answer would be.",
        "- `rationale` is one sentence naming the evidence you used.",
        "- Never answer for a case id you were not given.",
    ]
    .join("\n")
}

/// Render one packet as the user message. Text, not JSON: the model reads key/value lines without a
/// schema-acrobatics step, and the case ids are copied verbatim from the store.
pub(super) fn render_packet(packet: &ReviewPacket) -> String {
    let capacity = packet
        .cases
        .len()
        .saturating_add(packet.evidence.len())
        .saturating_add(4);
    let mut lines = Vec::with_capacity(capacity);
    lines.push(format!("subject_id: {}", packet.subject_id));
    lines.push(format!("subject: {}", packet.subject));
    lines.push(String::new());
    lines.push("cases:".to_string());
    for case in &packet.cases {
        lines.push(format!(
            "- case_id: {}\n  family: {}\n  detail: {}",
            case.case_id, case.family, case.detail
        ));
    }
    lines.push(String::new());
    lines.push("evidence:".to_string());
    for fact in &packet.evidence {
        lines.push(format!(
            "- {}: {} = {}",
            fact.source, fact.field, fact.value
        ));
    }
    lines.join("\n")
}

/// The schema every answer must satisfy.
pub(super) fn verdict_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "subject_id": { "type": "string" },
            "verdicts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "case_id": { "type": "string" },
                        "kind": {
                            "type": "string",
                            "enum": ["value_proposed", "insufficient_evidence"],
                        },
                        "field": { "type": "string" },
                        "value": { "type": "string" },
                        "confidence": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rationale": { "type": "string" },
                    },
                    "required": [
                        "case_id", "kind", "field", "value", "confidence", "rationale",
                    ],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["subject_id", "verdicts"],
        "additionalProperties": false,
    })
}
