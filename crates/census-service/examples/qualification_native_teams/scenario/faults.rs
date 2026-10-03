use anyhow::{ensure, Context, Result};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_service::restate_services::TeamsSourceRequest;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

use super::super::artifacts::{now, write_json};
use super::super::config::Config;
use super::super::process::OwnedProcess;
use super::super::proxy::RefusalProxy;
use super::{source, Run, SourceRepeat, SourceRun};

mod interruption;
pub use interruption::Interrupted;

#[derive(Serialize, Default)]
pub struct Faults {
    pub permanent: Option<Permanent>,
    pub interruption: Option<Interrupted>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct Permanent {
    pub input: TeamsSourceRequest,
    pub source: SourceRun,
    pub replay: SourceRepeat,
    pub before: Vec<Value>,
    pub after: Vec<Value>,
    pub started_at: String,
    pub ended_at: String,
}

#[tracing::instrument(skip_all)]
pub(super) async fn run(
    config: &Config,
    client: &Client,
    proxy: &RefusalProxy,
    endpoint: &mut OwnedProcess,
    first: &Run,
    body: &Value,
) -> Result<Faults> {
    let mut faults = Faults::default();
    let baseline = first
        .sources
        .iter()
        .find(|source| source.source == "milesplit")
        .context("native milesplit baseline child absent")?;
    ensure!(
        super::super::artifacts::rows(&first.invocation)?
            .iter()
            .all(terminal),
        "source fault isolation requires terminal original parent"
    );
    ensure!(
        super::super::artifacts::rows(&first.source_invocations)?
            .iter()
            .all(terminal),
        "source fault isolation requires terminal original children"
    );
    let quiescent = super::super::http::query(client, &config.root, &config.admin,
        "SELECT id, status, target_service_name, target_service_key FROM sys_invocation WHERE target_service_name IN ('JurisdictionCensus', 'TeamsSource') AND target_handler_name = 'run' AND status NOT IN ('completed', 'killed')").await?;
    write_json(
        &config.root.join("source-fault-quiescence.json"),
        &quiescent,
    )?;
    ensure!(
        super::super::artifacts::rows(&quiescent)?.is_empty(),
        "source-only fault isolation requires no active native parent or source execution"
    );
    let input = source::input(body, &first.state, &baseline.source)?;
    match permanent(config, client, proxy, &input).await {
        Ok(witness) => faults.permanent = Some(witness),
        Err(error) => faults
            .errors
            .push(format!("permanent policy witness: {error:#}")),
    }
    match interruption::run(config, client, proxy, endpoint, &input).await {
        Ok(witness) => faults.interruption = Some(witness),
        Err(error) => faults
            .errors
            .push(format!("interruption witness: {error:#}")),
    }
    write_json(
        &config.root.join("source-faults.json"),
        &serde_json::to_value(&faults)?,
    )?;
    Ok(faults)
}

fn terminal(row: &Value) -> bool {
    row.get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| matches!(status, "completed" | "killed"))
}

fn revised(input: &TeamsSourceRequest, revision: u32) -> (String, TeamsSourceRequest) {
    let mut input = input.clone();
    input.jurisdiction.revision = Revision(revision);
    let parent = WorkflowIdentity::jurisdiction(
        input.jurisdiction.jurisdiction,
        input.jurisdiction.season,
        input.jurisdiction.revision,
    );
    (format!("{}/teams/{}", parent.as_str(), input.source), input)
}

#[tracing::instrument(skip_all)]
async fn permanent(
    config: &Config,
    client: &Client,
    proxy: &RefusalProxy,
    input: &TeamsSourceRequest,
) -> Result<Permanent> {
    let (key, mut input) = revised(input, 2);
    input.jurisdiction.source_parallelism = 2;
    let started_at = now()?;
    let before = proxy.observations()?;
    let virgin = source::progress(config, client, &key).await?;
    ensure!(
        virgin.body.as_array().is_some_and(Vec::is_empty),
        "permanent source key is not fresh"
    );
    let id = source::submit(config, client, &key, &input).await?;
    super::wait_terminal(config, client, &id).await?;
    let row = source::target(config, client, &key, &id).await?;
    let source = source::capture(config, client, &input.source, &key, &id, row).await?;
    let replay = source::repeat(config, client, proxy, &source, input.clone()).await?;
    let witness = Permanent {
        input,
        source,
        replay,
        before,
        after: proxy.observations()?,
        started_at,
        ended_at: now()?,
    };
    write_json(
        &config.root.join("permanent-policy-witness.json"),
        &serde_json::to_value(&witness)?,
    )?;
    Ok(witness)
}
