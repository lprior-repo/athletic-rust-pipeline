use super::super::input::{self, Original};
use super::super::observe::{self, Observation};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::JurisdictionReport;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ParentRecovery {
    Completed {
        report: JurisdictionReport,
    },
    PausedResumable {
        pause: super::quiescence::TreePause,
        remaining: Value,
    },
}

impl ParentRecovery {
    pub(super) fn verify(&self, original: &Original, observation: &Observation) -> Result<()> {
        let parent = observe::status(&observation.after, &original.id)?;
        match self {
            Self::Completed { report } => {
                ensure!(
                    input::text(parent, "status")? == "completed"
                        && input::text(parent, "completion_result")? == "success",
                    "parent completion is unproven"
                );
                ensure!(
                    report.identity == original.key
                        && report.jurisdiction == original.request.jurisdiction
                        && report.history_window == original.request.history,
                    "reattached output scope differs"
                );
            }
            Self::PausedResumable { pause, remaining } => {
                ensure!(
                    input::text(parent, "status")? == "paused",
                    "resumable parent is not actually paused"
                );
                pause.verify(original)?;
                ensure!(
                    *remaining == super::obligations(observation, original)?,
                    "resumable parent remaining obligations differ"
                );
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, client, original))]
    pub(super) async fn live_tree(&self, client: &Client, original: &Original) -> Result<Value> {
        let current = super::quiescence::inventory(client, original).await?;
        match self {
            Self::Completed { .. } => ensure!(
                current
                    .iter()
                    .all(|row| row.get("status").and_then(Value::as_str) == Some("completed")),
                "completed parent retains active descendant work"
            ),
            Self::PausedResumable { pause, .. } => ensure!(
                current == pause.after,
                "paused original invocation tree changed across measured interval"
            ),
        }
        Ok(serde_json::to_value(current)?)
    }
}

#[tracing::instrument(skip(client, original, observation, control))]
pub(super) async fn outcome(
    client: &Client,
    original: &Original,
    observation: &Observation,
    control: super::quiescence::Control,
) -> Result<ParentRecovery> {
    let parent = observe::status(&observation.after, &original.id)?;
    let outcome = match control {
        super::quiescence::Control::Completed => {
            ensure!(
                input::text(parent, "status")? == "completed"
                    && input::text(parent, "completion_result")? == "success",
                "original recovered with terminal failure: {}",
                parent
                    .get("completion_failure")
                    .context("terminal failure reason absent")?
            );
            let output = super::super::super::http::request(
                client,
                Method::GET,
                &format!(
                    "{}restate/attach/{}",
                    super::super::super::http::INGRESS,
                    original.id
                ),
                None,
            )
            .await?;
            ParentRecovery::Completed {
                report: serde_json::from_value(output)?,
            }
        }
        super::quiescence::Control::Paused(pause) => ParentRecovery::PausedResumable {
            pause,
            remaining: super::obligations(observation, original)?,
        },
    };
    outcome.verify(original, observation)?;
    Ok(outcome)
}
