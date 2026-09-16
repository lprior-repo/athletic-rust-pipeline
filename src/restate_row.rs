use super::{
    analysis::{self, Evidence, Failure},
    step, Runtime,
};
use crate::{
    address, exhaustive_identity as identity,
    model::MatchRecord,
    restate_types::{self, RowInput, RowIssue, RowOutput},
};
use restate_sdk::prelude::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) struct AthleteRow {
    pub runtime: Arc<Runtime>,
}
const MAX_ROW_ATTEMPTS: u32 = 64;

#[restate_sdk::object(
    lazy_state = true,
    inactivity_timeout = "10m",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 10, on_max_attempts = "pause")
)]
impl AthleteRow {
    #[handler]
    async fn process(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<RowInput>,
    ) -> Result<Json<RowOutput>, HandlerError> {
        self.validate(&ctx, &input.0)?;
        bind_input(&ctx, &input.0).await?;
        let previous = step::cached::<RowOutput>(&ctx, "outcome").await?;
        if let Some(output) = &previous {
            if restate_types::final_record(&output.record) || output.attempt >= MAX_ROW_ATTEMPTS {
                return Ok(Json(output.clone()));
            }
        }
        let attempt = previous
            .as_ref()
            .map_or(0, |output| output.attempt)
            .checked_add(1)
            .ok_or_else(|| TerminalError::new("row attempt counter overflow"))?;
        let address = address::parse(&input.0.prospect);
        let mut issues = previous.map_or_else(Vec::new, |output| output.issues);
        if attempt == 1 {
            issues.extend(address.issues().iter().map(|issue| RowIssue {
                stage: "address".to_owned(),
                code: issue.code.clone(),
                message: issue.message.clone(),
                retryable: false,
                attempt,
            }));
        }
        let mut record = self
            .run_attempt(&ctx, &input.0, attempt, &mut issues)
            .await?;
        if address.requires_review() && matches!(record.status.as_str(), "MATCH" | "CLOSE_MATCH") {
            record.status = "REVIEW".to_owned();
            identity::clear_attribution(&mut record);
            record
                .notes
                .push_str("; Address syntax requires review; no residence was inferred");
            record.ai_logic.push_str("; Address syntax requires review");
        }
        if attempt == MAX_ROW_ATTEMPTS && !restate_types::final_record(&record) {
            issues.push(RowIssue { stage: "execution".to_owned(), code: "ROW_ATTEMPTS_EXHAUSTED".to_owned(),
                message: "64 explicit row attempts exhausted; retained unresolved for operator intervention".to_owned(), retryable: false, attempt });
        }
        record.processed_at_unix = step::timestamp(&ctx).await?;
        let output = RowOutput {
            record,
            address,
            issues,
            attempt,
        };
        ctx.set("outcome", Json(output.clone()));
        Ok(Json(output))
    }

    #[handler]
    async fn status(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<Option<RowOutput>>, HandlerError> {
        Ok(Json(
            ctx.get::<Json<RowOutput>>("outcome")
                .await?
                .map(|output| output.0),
        ))
    }
}

impl AthleteRow {
    fn validate(&self, ctx: &ObjectContext<'_>, input: &RowInput) -> Result<(), TerminalError> {
        self.runtime.validate(&input.config_digest)?;
        if input.run_fingerprint.len() != 64
            || !input
                .run_fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || input.prospect.source_key
                != format!("{}:{}", input.prospect.sheet, input.prospect.excel_row)
            || ctx.key()
                != restate_types::row_key(&input.run_fingerprint, &input.prospect.source_key)
        {
            return Err(TerminalError::new("invalid immutable row identity"));
        }
        if self.runtime.config.retrieval.authorized_direct_fetch && !input.authorization_ack {
            return Err(TerminalError::new(
                "direct retrieval requires explicit authorization",
            ));
        }
        if !input.no_ai
            && (!self.runtime.config.ollama.enabled
                || !self
                    .runtime
                    .config
                    .identity_review
                    .as_ref()
                    .is_some_and(|model| model.enabled))
        {
            return Err(TerminalError::new(
                "AI mode requires enabled extraction and review models",
            ));
        }
        Ok(())
    }

    async fn run_attempt(
        &self,
        ctx: &ObjectContext<'_>,
        input: &RowInput,
        attempt: u32,
        issues: &mut Vec<RowIssue>,
    ) -> Result<MatchRecord, HandlerError> {
        if input.prospect.full_name().trim().is_empty() {
            issues.push(RowIssue {
                stage: "input".to_owned(),
                code: "MISSING_NAME".to_owned(),
                message: "Neither source name is present".to_owned(),
                retryable: false,
                attempt,
            });
            return Ok(identity::error_record(
                &input.prospect,
                Vec::new(),
                "INPUT_ERROR",
                "Missing name".to_owned(),
            ));
        }
        let mut evidence = Evidence::default();
        let result = analysis::analyze(ctx, &self.runtime, input, &mut evidence).await;
        let mut record = match result {
            Ok(decision) => identity::finalize(
                input.prospect.clone(),
                evidence.candidates,
                decision,
                &self.runtime.config.matching,
                self.runtime.config.discovery.ambiguity_margin,
            ),
            Err(Failure::Execution(error)) => return Err(error.into()),
            Err(Failure::Domain(stage, error)) => {
                issues.push(RowIssue {
                    stage: stage.to_owned(),
                    code: error.code,
                    message: error.message.clone(),
                    retryable: error.retryable,
                    attempt,
                });
                let status = if matches!(stage, "extraction" | "review") {
                    "AI_ERROR"
                } else {
                    "SEARCH_ERROR"
                };
                identity::error_record(&input.prospect, evidence.candidates, status, error.message)
            }
        };
        record.deterministic_decision = evidence.deterministic;
        Ok(record)
    }
}

async fn bind_input(ctx: &ObjectContext<'_>, input: &RowInput) -> Result<(), HandlerError> {
    let bytes = serde_json::to_vec(input).map_err(|error| TerminalError::new(error.to_string()))?;
    let fingerprint = format!("{:x}", Sha256::digest(bytes));
    match ctx.get::<String>("input-binding").await? {
        Some(expected) if expected != fingerprint => {
            Err(TerminalError::new("row payload differs from durable input binding").into())
        }
        Some(_) => Ok(()),
        None => {
            ctx.set("input-binding", fingerprint);
            Ok(())
        }
    }
}
