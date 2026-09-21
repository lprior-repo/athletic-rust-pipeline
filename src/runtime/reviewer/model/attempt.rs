use super::AssistantVerdict;
use crate::runtime::protocol::{DocumentReceipt, FailureCode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Attempt {
    Success {
        verdict: AssistantVerdict,
        receipt: DocumentReceipt,
    },
    Failure {
        code: FailureCode,
        message: String,
        status: Option<u16>,
        receipt: Option<DocumentReceipt>,
        retryable: bool,
        retry_after_ms: u64,
    },
}

impl Attempt {
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Failure {
                retryable: true,
                ..
            }
        )
    }

    pub fn receipt(&self) -> Option<&DocumentReceipt> {
        match self {
            Self::Success { receipt, .. } => Some(receipt),
            Self::Failure { receipt, .. } => receipt.as_ref(),
        }
    }

    pub fn retry_after_ms(&self) -> u64 {
        match self {
            Self::Success { .. } => 0,
            Self::Failure { retry_after_ms, .. } => *retry_after_ms,
        }
    }
}
