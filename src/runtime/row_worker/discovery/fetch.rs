use super::super::MAX_PROFILE_BYTES_PER_ROW;
use crate::{
    domain::identity::EvidenceDigest,
    runtime::{acquisition::ProfileJob, profile_worker::ProfileWorkerClient, Runtime},
};
use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub(super) async fn identify(
    ctx: &ObjectContext<'_>,
    job: &ProfileJob,
) -> std::result::Result<EvidenceDigest, TerminalError> {
    let key = job
        .key()
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let call = ctx
        .object_client::<ProfileWorkerClient>(&key)
        .identify(Json(job.clone()))
        .call();
    let handle = call
        .invocation_handle()
        .await
        .map_err(|error| TerminalError::new(error.to_string()))?;
    match call.await {
        Ok(value) => Ok(value.0),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error)
        }
        Err(error) => Err(error),
    }
}

pub(super) async fn gather(
    ctx: &ObjectContext<'_>,
    job: &ProfileJob,
) -> std::result::Result<EvidenceDigest, TerminalError> {
    let key = job
        .key()
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let call = ctx
        .object_client::<ProfileWorkerClient>(&key)
        .gather(Json(job.clone()))
        .call();
    let handle = call
        .invocation_handle()
        .await
        .map_err(|error| TerminalError::new(error.to_string()))?;
    match call.await {
        Ok(value) => Ok(value.0),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error)
        }
        Err(error) => Err(error),
    }
}

pub(super) enum ProfileLoad<T> {
    Loaded { artifact: Box<T>, bytes: usize },
    Limited { bytes: usize },
}

pub(super) async fn load_artifact<T: DeserializeOwned + Send + 'static>(
    runtime: Arc<Runtime>,
    digest: &EvidenceDigest,
    used: usize,
) -> Result<ProfileLoad<T>> {
    let store = runtime.store.clone();
    let digest = digest.clone();
    runtime
        .blocking(move || {
            let bytes = store.get_bytes(&digest)?;
            if bytes.len() > MAX_PROFILE_BYTES_PER_ROW
                || used
                    .checked_add(bytes.len())
                    .is_none_or(|total| total > MAX_PROFILE_BYTES_PER_ROW)
            {
                return Ok(ProfileLoad::Limited { bytes: bytes.len() });
            }
            let artifact = serde_json::from_slice(&bytes).context("decoding profile artifact")?;
            Ok(ProfileLoad::Loaded {
                artifact: Box::new(artifact),
                bytes: bytes.len(),
            })
        })
        .await
}
