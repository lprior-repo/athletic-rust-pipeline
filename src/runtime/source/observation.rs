use super::{http::AttemptResult, request::RequestSpec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CapturedAttempt {
    pub(crate) request: RequestSpec,
    #[serde(flatten)]
    pub(crate) result: AttemptResult,
}
