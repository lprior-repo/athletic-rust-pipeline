mod attempt;
mod prompt;
mod request;
mod response;
mod validation;
mod verdict;

pub use attempt::Attempt;
pub use prompt::build_request;
pub use request::{ChatMessage, ChatRequest, ResponseFormat, ThinkingOptions};
pub use response::parse_response;
pub use verdict::{AssistantEvidenceRef, AssistantVerdict};

#[cfg(feature = "fuzzing")]
pub use response::fuzz_parse_response;

#[cfg(test)]
mod tests;
