use anyhow::{ensure, Context, Result};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_service::restate_services::JurisdictionRequest;
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::future::Future;
use std::time::Duration;

use super::artifacts::{rows, write_json};
use super::config::Config;
use super::http::{self, Observation};

mod faults;
mod snapshot;
mod source;

pub use faults::{Faults, Interrupted, Permanent};
pub use source::{SourceRepeat, SourceRun};

pub struct Run {
    pub id: String,
    pub state: Value,
    pub output: Observation,
    pub journal: Value,
    pub events: Value,
    pub admin_state: Value,
    pub status: Value,
    pub invocation: Value,
    pub physical: Vec<Value>,
    pub sources: Vec<SourceRun>,
    pub source_invocations: Value,
}

pub fn request() -> Result<(String, Value)> {
    let jurisdiction = UsJurisdiction::from_code("OH").context("OH jurisdiction absent")?;
    let season = SchoolYear::new(2026).context("2026 school year absent")?;
    let revision = Revision(1);
    let identity = WorkflowIdentity::jurisdiction(jurisdiction, season, revision);
    let request = JurisdictionRequest {
        jurisdiction,
        season,
        revision,
        history: census_service::restate_services::HistoryWindow::cohort(
            &census_crawl::net::today_iso(),
        )?,
        refresh: true,
        limit_per_state: Some(1),
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    Ok((
        identity.as_str().to_string(),
        serde_json::to_value(request)?,
    ))
}

#[tracing::instrument(skip(config, client))]
pub async fn register(config: &Config, client: &Client) -> Result<()> {
    until(120, || async {
        let result = http::request(
            client,
            &config.root,
            Method::GET,
            &format!("{}deployments", config.admin),
            None,
        )
        .await;
        Ok(result
            .ok()
            .filter(|response| (200..300).contains(&response.status)))
    })
    .await?;
    let registration = until(120, || async {
        let result = http::request(
            client,
            &config.root,
            Method::POST,
            &format!("{}deployments", config.admin),
            Some(&json!({"uri":format!("http://127.0.0.1:{}/", config.endpoint_port)})),
        )
        .await;
        Ok(result
            .ok()
            .filter(|response| (200..300).contains(&response.status)))
    })
    .await?;
    write_json(
        &config.root.join("registration.json"),
        &serde_json::to_value(registration)?,
    )?;
    let services = http::query(
        client,
        &config.root,
        &config.admin,
        "SELECT name, revision, ty FROM sys_service",
    )
    .await?;
    let registered = rows(&services)?;
    ensure!(
        registered
            .iter()
            .any(|row| row.get("name").and_then(Value::as_str) == Some("JurisdictionCensus")),
        "actual JurisdictionCensus not registered"
    );
    ensure!(
        registered.iter().any(
            |row| row.get("name").and_then(Value::as_str) == Some("TeamsSource")
                && row.get("ty").and_then(Value::as_str) == Some("virtual_object")
        ),
        "mandatory TeamsSource virtual object not registered"
    );
    write_json(&config.root.join("registered-services.json"), &services)
}

#[tracing::instrument(skip(config, client, body))]
pub async fn run(
    config: &Config,
    client: &Client,
    key: &str,
    body: &Value,
    label: &str,
) -> Result<Run> {
    let accepted = http::request(
        client,
        &config.root,
        Method::POST,
        &format!("{}JurisdictionCensus/{key}/run/send", config.ingress),
        Some(body),
    )
    .await?;
    ensure!(
        (200..300).contains(&accepted.status),
        "native production submission refused: {} {}",
        accepted.status,
        accepted.body
    );
    let id = accepted
        .body
        .get("invocationId")
        .and_then(Value::as_str)
        .context("actual native invocationId missing")?
        .to_string();
    validate_id(&id)?;
    let status = parent_boundary(config, client, &id).await?;
    let run = snapshot::capture(config, client, key, &id, status).await?;
    write_json(
        &config.root.join(format!("{label}-native-snapshot.json")),
        &json!({"invocation_id":run.id, "state":run.state, "output":run.output, "journal":run.journal, "events":run.events, "admin_state":run.admin_state, "store_status":run.status, "source_invocations":run.source_invocations, "sources":run.sources}),
    )?;
    Ok(run)
}

pub fn validate_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "invalid native invocation ID"
    );
    Ok(())
}

#[tracing::instrument(skip(config, client, proxy, first, body))]
pub async fn repeat_sources(
    config: &Config,
    client: &Client,
    proxy: &super::proxy::RefusalProxy,
    first: &Run,
    body: &Value,
) -> Result<Vec<SourceRepeat>> {
    let parents = rows(&first.invocation)?;
    let quiescent = rows(&first.source_invocations)?.iter().all(|row| {
        row.get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| matches!(status, "completed" | "killed"))
    }) && !parents.is_empty()
        && parents.iter().all(|row| {
            row.get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| matches!(status, "completed" | "killed" | "paused"))
        });
    if !quiescent {
        write_json(
            &config.root.join("source-repeat-unproven.json"),
            &json!({"status":"UNPROVEN", "reason":"parent or child native invocation not quiescent; source-only physical attribution unavailable"}),
        )?;
        return Ok(Vec::new());
    }
    stream::iter(
        first
            .sources
            .iter()
            .filter(|source| source.outcome.is_some()),
    )
    .then(|source| async {
        let input = source::input(body, &first.state, &source.source)?;
        source::repeat(config, client, proxy, source, input).await
    })
    .try_collect()
    .await
}

#[tracing::instrument(skip_all)]
pub async fn source_faults(
    config: &Config,
    client: &Client,
    proxy: &super::proxy::RefusalProxy,
    endpoint: &mut super::process::OwnedProcess,
    first: &Run,
    body: &Value,
) -> Faults {
    match faults::run(config, client, proxy, endpoint, first, body).await {
        Ok(faults) => faults,
        Err(error) => Faults {
            errors: vec![format!("source fault orchestration: {error:#}")],
            ..Faults::default()
        },
    }
}

#[tracing::instrument(skip(config, client))]
async fn parent_boundary(config: &Config, client: &Client, id: &str) -> Result<Value> {
    match tokio::time::timeout(Duration::from_secs(60), wait_terminal(config, client, id)).await {
        Ok(result) => result,
        Err(error) => {
            write_json(
                &config.root.join(format!("{id}-parent-poll-boundary.json")),
                &json!({"status":"UNPROVEN", "reason":"parent did not settle within native polling boundary",
                    "error":error.to_string(), "invocation_id":id}),
            )?;
            http::query(client, &config.root, &config.admin,
                &format!("SELECT id, status, completion_result, completion_failure FROM sys_invocation WHERE id = '{id}'")).await
        }
    }
}

#[tracing::instrument(skip(config, client))]
async fn wait_terminal(config: &Config, client: &Client, id: &str) -> Result<Value> {
    until(600, || async {
        let query = http::query(client, &config.root, &config.admin, &format!("SELECT id, status, completion_result, completion_failure FROM sys_invocation WHERE id = '{id}'")).await?;
        let terminal = rows(&query)?.iter().any(|row| row.get("status").and_then(Value::as_str).is_some_and(|status| matches!(status, "completed" | "killed" | "paused")));
        Ok(terminal.then_some(query))
    }).await
}

#[tracing::instrument(skip(check))]
async fn until<T, F, Fut>(attempts: usize, check: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<T>>>,
{
    let mut check = check;
    let candidates = stream::iter(0..attempts)
        .then(move |_| {
            let future = check();
            async move {
                tokio::time::sleep(Duration::from_millis(250)).await;
                future.await
            }
        })
        .try_filter_map(|candidate| futures::future::ready(Ok(candidate)));
    futures::pin_mut!(candidates);
    tokio::time::timeout(Duration::from_secs(180), candidates.try_next())
        .await
        .context("native polling exceeded 180-second budget")??
        .context("native polling exhausted fixed attempt budget")
}
