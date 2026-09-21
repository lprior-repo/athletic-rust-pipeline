use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: [ChatMessage; 2],
    pub temperature: f32,
    pub max_tokens: u16,
    pub reasoning_effort: &'static str,
    pub chat_template_kwargs: ThinkingOptions,
    pub response_format: ResponseFormat,
}

#[derive(Clone, Serialize)]
pub struct ChatMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Clone, Serialize)]
pub struct ThinkingOptions {
    pub enable_thinking: bool,
}

#[derive(Clone, Serialize)]
pub struct ResponseFormat {
    pub r#type: &'static str,
}
