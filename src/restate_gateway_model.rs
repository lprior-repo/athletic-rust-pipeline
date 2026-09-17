use super::{
    step::{self, EffectFailure},
    Runtime,
};
use crate::{
    extract::{self, OllamaClient},
    fetch,
    model::{Candidate, ModelDecision, Prospect, SearchHit},
};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ExtractionJob {
    pub digest: String,
    pub prospect: Prospect,
    pub hit: SearchHit,
    pub html: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ReviewJob {
    pub digest: String,
    pub prospect: Prospect,
    pub candidates: Vec<Candidate>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ProfileJob {
    pub digest: String,
    pub url: String,
    pub authorization_ack: bool,
}

pub(super) struct AthleticModels {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "10m",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 10, on_max_attempts = "pause")
)]
impl AthleticModels {
    #[handler]
    async fn extract(
        &self,
        ctx: ObjectContext<'_>,
        job: Json<ExtractionJob>,
    ) -> Result<Json<Result<Candidate, EffectFailure>>, HandlerError> {
        self.validate(&ctx, &job.0.digest, "extractor")?;
        let result = step::journal(&ctx, "Q5-extraction", || async {
            let client = OllamaClient::new(&self.runtime.config.ollama).map_err(model_failure)?;
            if !client.is_enabled() {
                return Err(EffectFailure::new(
                    "MODEL_DISABLED",
                    "extraction model disabled",
                    false,
                ));
            }
            extract::candidate_from_evidence_required(
                &job.0.prospect,
                &job.0.hit,
                job.0.html.as_deref(),
                &client,
                self.runtime.config.retrieval.page_text_limit,
            )
            .await
            .map_err(model_failure)
        })
        .await?;
        Ok(Json(result))
    }

    #[handler]
    async fn review(
        &self,
        ctx: ObjectContext<'_>,
        job: Json<ReviewJob>,
    ) -> Result<Json<Result<ModelDecision, EffectFailure>>, HandlerError> {
        self.validate(&ctx, &job.0.digest, "reviewer")?;
        let result = step::journal(&ctx, "Q4-review", || async {
            let config = self
                .runtime
                .config
                .identity_review
                .as_ref()
                .ok_or_else(|| {
                    EffectFailure::new("MODEL_DISABLED", "identity reviewer not configured", false)
                })?;
            let client = OllamaClient::new(config).map_err(model_failure)?;
            if !client.is_enabled() {
                return Err(EffectFailure::new(
                    "MODEL_DISABLED",
                    "identity reviewer disabled",
                    false,
                ));
            }
            client
                .validate_identity_required(&job.0.prospect, &job.0.candidates)
                .await
                .map_err(model_failure)
        })
        .await?;
        Ok(Json(result))
    }

    #[handler]
    async fn profile(
        &self,
        ctx: ObjectContext<'_>,
        job: Json<ProfileJob>,
    ) -> Result<Json<Result<Option<String>, EffectFailure>>, HandlerError> {
        self.validate(&ctx, &job.0.digest, "retrieval")?;
        if self.runtime.config.retrieval.authorized_direct_fetch && !job.0.authorization_ack {
            return Err(
                TerminalError::new("direct retrieval requires explicit authorization").into(),
            );
        }
        let result =
            step::journal(&ctx, "profile-evidence", || self.profile_html(&job.0.url)).await?;
        Ok(Json(result))
    }
}

impl AthleticModels {
    fn validate(
        &self,
        ctx: &ObjectContext<'_>,
        digest: &str,
        key: &str,
    ) -> Result<(), TerminalError> {
        self.runtime.validate(digest)?;
        if ctx.key() != key {
            return Err(TerminalError::new("invalid model admission key"));
        }
        Ok(())
    }

    async fn profile_html(&self, url: &str) -> Result<Option<String>, EffectFailure> {
        if let Some(directory) = &self.runtime.config.retrieval.saved_pages_dir {
            let directory = directory.clone();
            let owned_url = url.to_owned();
            let task = tokio::task::spawn_blocking(move || {
                fetch::load_saved_profile(&owned_url, &directory)
            });
            let html = task
                .await
                .map_err(|error| EffectFailure::new("RETRIEVAL_TASK", error.to_string(), true))?
                .map_err(retrieval_failure)?;
            if html.is_some() {
                return Ok(html);
            }
        }
        Ok(None)
    }
}

fn model_failure(error: anyhow::Error) -> EffectFailure {
    EffectFailure::new("AI_ERROR", format!("{error:#}"), true)
}
fn retrieval_failure(error: anyhow::Error) -> EffectFailure {
    EffectFailure::new("RETRIEVAL_ERROR", format!("{error:#}"), true)
}
