use super::AssistantVerdict;
use crate::runtime::protocol::ReviewInput;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
    refusal: Option<String>,
}

pub fn parse_response(bytes: &[u8], input: &ReviewInput) -> Result<AssistantVerdict> {
    let response: OpenAiResponse = serde_json::from_slice(bytes)
        .map_err(|error| anyhow!("malformed local model response: {error}"))?;
    if response.choices.len() != 1 {
        bail!("local model response must contain exactly one choice");
    }
    let choice = response
        .choices
        .first()
        .ok_or_else(|| anyhow!("missing model choice"))?;
    if choice.finish_reason.as_deref() != Some("stop") {
        bail!("local model response did not finish normally");
    }
    if choice.message.refusal.is_some() {
        bail!("local model refused the review");
    }
    let content = choice
        .message
        .content
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("local model response has no content"))?;
    let verdict: AssistantVerdict = serde_json::from_str(content)
        .map_err(|error| anyhow!("malformed review decision: {error}"))?;
    verdict.validate(input)
}

/// Fuzz-only parser entrypoint with a fixed, valid review input.
#[cfg(feature = "fuzzing")]
pub fn fuzz_parse_response(bytes: &[u8]) -> Result<AssistantVerdict> {
    static INPUT: std::sync::LazyLock<std::result::Result<ReviewInput, serde_json::Error>> =
        std::sync::LazyLock::new(|| {
            serde_json::from_value(serde_json::json!({
                "source": {
                    "source_key": "Sheet:2",
                    "sheet": "Sheet",
                    "excel_row": 2,
                    "fields": {"Person First": "Ada"}
                },
                "candidates": [{
                    "athlete_id": 7,
                    "name": "Ada Runner",
                    "teams": [{
                        "team_id": 9,
                        "name": {
                            "value": "Central",
                            "evidence": {
                                "document": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                                "locator": "team/name"
                            }
                        },
                        "location": null,
                        "seasons": [],
                        "level": null
                    }],
                    "graduation_years": [],
                    "sports": [],
                    "issues": [],
                    "eligibility_reasons": ["matching sport"],
                    "documents": []
                }]
            }))
        });
    let input = INPUT
        .as_ref()
        .map_err(|error| anyhow!("synthetic fuzz review input is invalid: {error}"))?;
    parse_response(bytes, input)
}
