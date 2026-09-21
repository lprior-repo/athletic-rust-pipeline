use super::super::{
    acquisition::{ProfileAcquisition, ProfileJob, ProfileProbe},
    protocol::{DocumentReceipt, FailureCode, OperationFailure, RetryEvidence},
    Runtime,
};
use crate::domain::{
    evidence::{EvidenceIssue, EvidenceRef},
    identity::EvidenceDigest,
};
use anyhow::Result;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(super) async fn publish(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    artifact: ProfileAcquisition,
) -> Result<Json<EvidenceDigest>, HandlerError> {
    let digest = ctx
        .run(|| async move { Ok(Json(runtime.store_json(artifact).await.map_err(terminal)?)) })
        .name("profile-acquisition-publication")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    ctx.set("result", Json(digest.0.clone()));
    Ok(digest)
}

pub(super) async fn publish_probe(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    probe: ProfileProbe,
) -> Result<Json<EvidenceDigest>, HandlerError> {
    let digest = ctx
        .run(|| async move { Ok(Json(runtime.store_json(probe).await.map_err(terminal)?)) })
        .name("profile-identity-probe-publication")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    ctx.set("probe", Json(digest.0.clone()));
    Ok(digest)
}

pub(super) fn validate_key(ctx: &ObjectContext<'_>, job: &ProfileJob) -> Result<(), HandlerError> {
    if job.key().map_err(terminal)? == ctx.key() {
        Ok(())
    } else {
        Err(terminal(
            "profile worker key does not bind snapshot and athlete",
        ))
    }
}
pub(super) fn failure(
    code: FailureCode,
    message: String,
    evidence: Vec<DocumentReceipt>,
) -> Result<OperationFailure, HandlerError> {
    Ok(OperationFailure {
        code,
        message,
        http_status: None,
        retries: RetryEvidence::NotAttempted,
        evidence,
    })
}
pub(super) fn issue(
    code: &str,
    message: &str,
    digest: &EvidenceDigest,
    locator: &str,
) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence: Some(EvidenceRef {
            document: digest.clone(),
            locator: locator.to_owned(),
        }),
    }
}
pub(super) fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
