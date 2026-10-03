mod artifacts;
mod captures;
mod config;
mod evidence;
mod http;
mod isolation;
mod ledger;
mod lifecycle;
mod process;
mod proxy;
mod scenario;
mod signals;
mod sources;

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

use artifacts::{now, write_json};
use config::Config;
use process::OwnedProcess;
use proxy::RefusalProxy;

pub fn execute() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut signals = {
        let _entered = runtime.enter();
        signals::ShutdownSignals::install()?
    };
    let mut config = Config::prepare()?;
    let result = execute_owned(&mut config, &runtime, &mut signals);
    if let Err(error) = &result {
        write_json(
            &config.root.join("qualification-error.json"),
            &json!({"status":"ERROR", "at":now()?, "model":artifacts::MODEL, "error":format!("{error:#}"), "verification":"actual attempted boundaries only; inspect retained logs and process ledger", "national_completeness":false}),
        )?;
    }
    result
}

fn execute_owned(
    config: &mut Config,
    runtime: &tokio::runtime::Runtime,
    signals: &mut signals::ShutdownSignals,
) -> Result<()> {
    let prerequisites = config.capability()?;
    captures::prepare(&config.root)?;
    sources::save(&config.root)?;
    let mut proxy = RefusalProxy::start(&config.root)?;
    let mut node = match config.start_node() {
        Ok(node) => node,
        Err(error) => {
            let proxy_cleanup = proxy.stop();
            return Err(error.context(format!("startup proxy cleanup: {proxy_cleanup:?}")));
        }
    };
    let mut endpoint = match config.start_endpoint() {
        Ok(endpoint) => endpoint,
        Err(error) => {
            let node_cleanup = node.stop();
            let proxy_cleanup = proxy.stop();
            return Err(error.context(format!(
                "startup cleanup node={node_cleanup:?} proxy={proxy_cleanup:?}"
            )));
        }
    };
    let measured = run_owned(config, &mut node, &mut endpoint, &proxy, runtime, signals);
    let cleanup = lifecycle::cleanup(config, &mut endpoint, &mut node, &mut proxy);
    finish_owned(config, prerequisites, measured, cleanup)
}

fn finish_owned(
    config: &Config,
    prerequisites: Value,
    measured: Result<evidence::Measurement>,
    cleanup: Result<Value>,
) -> Result<()> {
    let cold = match &cleanup {
        Ok(certificate) => ledger::capture(&config.root, certificate),
        Err(error) => Err(anyhow::anyhow!(
            "cold inspection withheld after unsuccessful cleanup: {error:#}"
        )),
    };
    let evaluated = match &measured {
        Ok(measured) => evidence::evaluate(&config.root, measured, cold.as_ref().ok()),
        Err(error) => Err(anyhow::anyhow!(
            "native scenario did not reach evidence evaluation: {error:#}"
        )),
    };
    let passed = evaluated
        .as_ref()
        .is_ok_and(|value| value.get("status").and_then(Value::as_str) == Some("PASS"))
        && cleanup.is_ok()
        && cold.is_ok();
    write_json(
        &config.root.join("qualification.json"),
        &json!({"status":if passed {"PASS"} else {"BLOCKED_OR_UNPROVEN"}, "model":artifacts::MODEL,
            "ended":now()?, "prerequisites":prerequisites, "qualification":evaluated.as_ref().ok(),
            "scenario_error":measured.as_ref().err().map(|error| format!("{error:#}")),
            "evaluation_error":evaluated.as_ref().err().map(|error| format!("{error:#}")),
            "cleanup":cleanup.as_ref().ok(), "cleanup_error":cleanup.as_ref().err().map(|error| format!("{error:#}")),
            "cold_ledger_error":cold.as_ref().err().map(|error| format!("{error:#}")), "national_completeness":false}),
    )?;
    measured?;
    cleanup?;
    cold?;
    let result = evaluated?;
    ensure!(passed, "qualification reached measured BLOCKED_OR_UNPROVEN boundary: {}; inspect qualification-oracles.json",
        result.get("reached_boundary").map_or("unknown".to_string(), Value::to_string));
    println!(
        "NATIVE TEAMS QUALIFICATION PASS: {} (isolated qualification, not a national census)",
        config.root.display()
    );
    Ok(())
}

fn run_owned(
    config: &Config,
    node: &mut OwnedProcess,
    endpoint: &mut OwnedProcess,
    proxy: &RefusalProxy,
    runtime: &tokio::runtime::Runtime,
    signals: &mut signals::ShutdownSignals,
) -> Result<evidence::Measurement> {
    node.ensure_running()?;
    endpoint.ensure_running()?;
    let node_output = node.output_health();
    let endpoint_output = endpoint.output_health();
    runtime.block_on(async {
        tokio::select! {
            result = tokio::time::timeout(Duration::from_secs(600), qualify(config, proxy, endpoint)) => result.context("native qualification exceeded 600-second execution budget")?,
            result = signals.requested() => {
                result?;
                anyhow::bail!("driver interrupted; proceeding to ordered TERM/drain/reap cleanup")
            },
            result = process::watch_output(node_output, endpoint_output) => {
                result?;
                anyhow::bail!("child output monitor ended without scenario completion")
            }
        }
    })
}

#[tracing::instrument(skip_all)]
async fn qualify(
    config: &Config,
    proxy: &RefusalProxy,
    endpoint: &mut OwnedProcess,
) -> Result<evidence::Measurement> {
    let client = http::client()?;
    scenario::register(config, &client).await?;
    let (key, request) = scenario::request()?;
    write_json(
        &config.root.join("production-request.json"),
        &json!({"identity":key, "request":request, "fault_proxy":proxy.uri, "identity_reused":true, "ingress_idempotency_key_reused":false, "limit_per_state":1, "athlete_or_national_completeness":false}),
    )?;
    let mut first = scenario::run(config, &client, &key, &request, "first").await?;
    first.physical = proxy.observations()?;
    write_json(
        &config.root.join("first-physical-observations.json"),
        &json!(first.physical),
    )?;
    let source_repeats = scenario::repeat_sources(config, &client, proxy, &first, &request).await?;
    let attempted = scenario::run(config, &client, &key, &request, "repeat").await;
    let (repeat, repeat_error) = match attempted {
        Ok(mut repeat) => {
            repeat.physical = proxy.observations()?;
            write_json(
                &config.root.join("repeat-physical-observations.json"),
                &json!(repeat.physical),
            )?;
            (Some(repeat), None)
        }
        Err(error) => {
            let error = format!("{error:#}");
            write_json(
                &config.root.join("parent-repeat-unproven.json"),
                &json!({"status":"UNPROVEN", "error":error, "physical":proxy.observations()?}),
            )?;
            (None, Some(error))
        }
    };
    let faults = scenario::source_faults(config, &client, proxy, endpoint, &first, &request).await;
    Ok(evidence::Measurement {
        first,
        repeat,
        repeat_error,
        source_repeats,
        request,
        faults,
    })
}
