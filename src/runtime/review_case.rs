use super::{
    protocol::{ReviewInput, ReviewJob, ReviewOutcome, MAX_REVIEW_INPUT_BYTES},
    reviewer::LocalReviewerClient,
    ModelLane, Runtime, WorkerConfig,
};
use crate::domain::identity::EvidenceDigest;
use anyhow::{bail, Context, Result};
use restate_sdk::prelude::*;
use serde::Serialize;
use std::{collections::BTreeSet, sync::Arc};

const REVIEW_PROTOCOL_REVISION: &str = "review-protocol-workbook-eligible-v2";
const MAX_CANDIDATES: usize = 64;
const ASSIGNMENT_BUCKETS: u64 = 5;
const Q5_BUCKETS: u64 = 3;

/// Durable owner of one local reviewer assignment for an equivalent review case.
pub struct ReviewCase {
    pub runtime: Arc<Runtime>,
}

#[derive(Serialize)]
struct CaseIdentity<'a> {
    protocol_revision: &'static str,
    source_fields: &'a std::collections::BTreeMap<String, String>,
    candidates: &'a [super::protocol::ReviewCandidate],
    q5_model: &'a str,
    q4_model: &'a str,
}

/// Return the stable case key. Physical source location is deliberately not part of it.
pub fn case_key(config: &WorkerConfig, input: &ReviewInput) -> Result<String> {
    let identity = CaseIdentity {
        protocol_revision: REVIEW_PROTOCOL_REVISION,
        source_fields: &input.source.fields,
        candidates: &input.candidates,
        q5_model: config.model_id(ModelLane::Q5_5090),
        q4_model: config.model_id(ModelLane::Q4_3090),
    };
    Ok(super::identity::fingerprint(&identity)?.as_str().to_owned())
}

#[restate_sdk::object(
    ingress_private = true,
    lazy_state = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl ReviewCase {
    #[handler]
    pub async fn review(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<EvidenceDigest>,
    ) -> Result<Json<ReviewOutcome>, HandlerError> {
        let digest = input.into_inner();
        let review_input = load_review_input(self.runtime.clone(), &digest)
            .await
            .map_err(terminal)?;
        validate_input(&review_input).map_err(terminal)?;
        let expected_key = case_key(&self.runtime.config, &review_input).map_err(terminal)?;
        if expected_key != ctx.key() {
            return Err(terminal("review case key does not bind the review input"));
        }

        let expected_lane = assigned_lane(&expected_key).map_err(terminal)?;
        let assigned = ctx
            .get::<Json<ModelLane>>("assignment")
            .await?
            .map(|value| value.0);
        let retained = ctx.get::<Json<ReviewOutcome>>("result").await?;
        if let Some(result) = retained {
            let lane = outcome_lane(&result.0);
            if assigned != Some(expected_lane) || lane != expected_lane {
                return Err(terminal(
                    "review case has contradictory durable result assignment",
                ));
            }
            return Ok(result);
        }

        let lane = match assigned {
            Some(lane) if lane == expected_lane => lane,
            Some(_) => {
                return Err(terminal(
                    "review case durable assignment contradicts its key",
                ))
            }
            None => {
                ctx.set("assignment", Json(expected_lane));
                expected_lane
            }
        };
        let outcome = ctx
            .object_client::<LocalReviewerClient>(lane.key())
            .review(Json(ReviewJob {
                input: digest,
                lane,
            }))
            .call()
            .await?;
        ctx.set("result", Json(outcome.0.clone()));
        Ok(outcome)
    }
}

async fn load_review_input(runtime: Arc<Runtime>, digest: &EvidenceDigest) -> Result<ReviewInput> {
    let store = runtime.store.clone();
    let digest = digest.clone();
    runtime
        .blocking(move || {
            let bytes = store.get_bytes(&digest)?;
            if bytes.len() > MAX_REVIEW_INPUT_BYTES {
                bail!("review input exceeds 64 KiB");
            }
            serde_json::from_slice(&bytes).context("decoding verified review input")
        })
        .await
}

fn validate_input(input: &ReviewInput) -> Result<()> {
    if !(2..=MAX_CANDIDATES).contains(&input.candidates.len()) {
        bail!("review requires between 2 and 64 supplied candidates");
    }
    let unique = input
        .candidates
        .iter()
        .map(|candidate| candidate.athlete_id)
        .collect::<BTreeSet<_>>();
    if unique.len() != input.candidates.len() {
        bail!("review candidates must have distinct athlete IDs");
    }
    if input
        .candidates
        .iter()
        .any(|candidate| candidate.eligibility_reasons.is_empty())
    {
        bail!("review candidate lacks eligibility explanations");
    }
    Ok(())
}

fn assigned_lane(case_key: &str) -> Result<ModelLane> {
    let prefix = case_key.get(..16).context("review case key is too short")?;
    let bucket = u64::from_str_radix(prefix, 16).context("review case key is not hexadecimal")?
        % ASSIGNMENT_BUCKETS;
    Ok(if bucket < Q5_BUCKETS {
        ModelLane::Q5_5090
    } else {
        ModelLane::Q4_3090
    })
}

fn outcome_lane(outcome: &ReviewOutcome) -> ModelLane {
    match outcome {
        ReviewOutcome::Reviewed { lane, .. } | ReviewOutcome::Failed { lane, .. } => *lane,
    }
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceRecord;
    use crate::runtime::protocol::ReviewCandidate;
    use serde_json::json;
    use std::path::Path;
    use tempfile::tempdir;

    fn config(path: &Path, q5: &str, q4: &str) -> WorkerConfig {
        let text = format!(
            "mode = \"fixture\"\nstorage_dir = \"{}\"\nsource_origin = \"http://127.0.0.1/\"\nsource_interval_ms = 0\nrequest_timeout_seconds = 1\ncpu_workers = 1\nrow_concurrency = 1\nq5_url = \"http://127.0.0.1/\"\nq5_model = \"{q5}\"\nq4_url = \"http://127.0.0.1/\"\nq4_model = \"{q4}\"\n",
            path.display()
        );
        let file = path.join("worker.toml");
        std::fs::write(&file, text).expect("synthetic worker config");
        WorkerConfig::load(&file).expect("synthetic worker config loads")
    }

    fn input(source_key: &str, sheet: &str, excel_row: u32) -> ReviewInput {
        let candidate = |id: u64| {
            serde_json::from_value::<ReviewCandidate>(json!({
                "athlete_id": id,
                "name": format!("Athlete {id}"),
                "teams": [], "graduation_years": [], "sports": [], "issues": [],
                "eligibility_reasons": ["supplied eligible candidate"], "documents": []
            }))
            .expect("synthetic candidate")
        };
        ReviewInput {
            source: SourceRecord {
                source_key: source_key.to_owned(),
                sheet: sheet.to_owned(),
                excel_row,
                fields: [("Person First".to_owned(), "Ada".to_owned())]
                    .into_iter()
                    .collect(),
            },
            candidates: vec![candidate(7), candidate(8)],
        }
    }

    #[test]
    fn physical_source_location_does_not_change_case() {
        let directory = tempdir().expect("temporary directory");
        let worker = config(directory.path(), "q5-a", "q4-a");
        assert_eq!(
            case_key(&worker, &input("sheet-a:2", "Sheet A", 2)).expect("first key"),
            case_key(&worker, &input("sheet-b:99", "Sheet B", 99)).expect("second key")
        );
    }

    #[test]
    fn identity_proof_and_model_changes_change_case() {
        let directory = tempdir().expect("temporary directory");
        let worker = config(directory.path(), "q5-a", "q4-a");
        let base = input("row", "Sheet", 2);
        let mut identity = base.clone();
        identity
            .source
            .fields
            .insert("Person Last".to_owned(), "Runner".to_owned());
        assert_ne!(
            case_key(&worker, &base).expect("base key"),
            case_key(&worker, &identity).expect("changed identity")
        );
        let mut proof = base.clone();
        proof.candidates[0].documents.push(
            serde_json::from_str(
                "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
            )
            .expect("synthetic evidence digest"),
        );
        assert_ne!(
            case_key(&worker, &base).expect("base key"),
            case_key(&worker, &proof).expect("changed proof")
        );
        let changed_models = config(directory.path(), "q5-b", "q4-a");
        assert_ne!(
            case_key(&worker, &base).expect("base key"),
            case_key(&changed_models, &base).expect("changed models")
        );
    }

    #[test]
    fn deterministic_weighted_assignment_covers_both_lanes() {
        let lanes = (0_u32..256)
            .map(|index| super::super::identity::fingerprint(&index).expect("synthetic case hash"))
            .map(|key| assigned_lane(key.as_str()).expect("valid case hash"))
            .fold((0_u32, 0_u32), |(q5, q4), lane| match lane {
                ModelLane::Q5_5090 => (q5 + 1, q4),
                ModelLane::Q4_3090 => (q5, q4 + 1),
            });
        assert!(lanes.0 > 0 && lanes.1 > 0);
        assert_eq!(
            assigned_lane("0000000000000000").expect("Q5 key"),
            ModelLane::Q5_5090
        );
        assert_eq!(
            assigned_lane("0000000000000003").expect("Q4 key"),
            ModelLane::Q4_3090
        );
        assert!(assigned_lane("invalid").is_err());
    }
}
