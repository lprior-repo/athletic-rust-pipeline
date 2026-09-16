use super::{protocol::RetryEvidence, Runtime};
use crate::{
    domain::{facts::RetryCount, identity::EvidenceDigest},
    store::AttemptEvidence,
};
use restate_sdk::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(crate) const AUDIT_FAILURE: u16 = 507;

pub(crate) fn operation_key(
    invocation: &str,
    effect: &str,
) -> Result<EvidenceDigest, HandlerError> {
    let mut hash = Sha256::new();
    hash.update(invocation.as_bytes());
    hash.update([0]);
    hash.update(effect.as_bytes());
    EvidenceDigest::parse(&format!("{:x}", hash.finalize())).terminal()
}

/// Call only from a Restate run closure. Failure is terminal: do not repeat an HTTP
/// effect merely because its evidence could not be published.
pub(crate) async fn record<T: Serialize + Send + 'static>(
    runtime: Arc<Runtime>,
    operation: EvidenceDigest,
    value: T,
) -> Result<EvidenceDigest, HandlerError> {
    let store = runtime.store.clone();
    runtime
        .blocking(move || Ok(store.record_attempt(&operation, &value)?))
        .await
        .map_err(|_| {
            TerminalError::new_with_code(AUDIT_FAILURE, "HTTP attempt evidence publication failed")
                .into()
        })
}

/// Call only from a Restate run closure so the resulting evidence snapshot is journaled.
pub(crate) async fn load<T: DeserializeOwned + Send + 'static>(
    runtime: Arc<Runtime>,
    operation: EvidenceDigest,
) -> Result<Vec<AttemptEvidence<T>>, HandlerError> {
    let store = runtime.store.clone();
    runtime
        .blocking(move || {
            store
                .attempt_digests(&operation)?
                .iter()
                .map(|digest| {
                    store
                        .read_attempt(&operation, digest)
                        .map_err(anyhow::Error::from)
                })
                .collect()
        })
        .await
        .map_err(|_| {
            TerminalError::new_with_code(AUDIT_FAILURE, "HTTP attempt evidence retrieval failed")
                .into()
        })
}

pub(crate) fn retry_evidence<T>(
    operation: EvidenceDigest,
    records: &[AttemptEvidence<T>],
) -> Result<RetryEvidence, HandlerError> {
    let observed_attempts = u32::try_from(records.len()).terminal()?;
    let maximum_retries = RetryCount::new(3).terminal()?;
    let attempts = records.iter().map(|record| record.digest.clone()).collect();
    Ok(RetryEvidence::SdkControlled {
        operation,
        maximum_retries,
        observed_attempts,
        attempts,
    })
}

pub(crate) fn unavailable_evidence(
    operation: EvidenceDigest,
) -> Result<RetryEvidence, HandlerError> {
    let maximum_retries = RetryCount::new(3).terminal()?;
    Ok(RetryEvidence::SdkEvidenceUnavailable {
        operation,
        maximum_retries,
    })
}
