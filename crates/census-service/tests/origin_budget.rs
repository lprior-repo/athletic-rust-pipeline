#![forbid(unsafe_code)]

#[path = "origin_budget/certificate.rs"]
mod certificate;
#[path = "origin_budget/fixture.rs"]
mod fixture;
#[path = "origin_budget/ledger.rs"]
mod ledger;
#[path = "origin_budget/native_api.rs"]
mod native_api;
#[path = "origin_budget/native_certificate.rs"]
mod native_certificate;
#[path = "origin_budget/native_process.rs"]
mod native_process;
#[path = "origin_budget/native_scenario.rs"]
mod native_scenario;
#[path = "origin_budget/native_workload.rs"]
mod native_workload;
#[path = "origin_budget/process.rs"]
mod process;
#[path = "origin_budget/scenario.rs"]
mod scenario;
#[path = "origin_budget/server.rs"]
mod server;

use anyhow::Result;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_native_workflows_do_not_multiply_the_physical_origin_budget() -> Result<()> {
    let directory = certificate::directory()?;
    let binary = native_process::pinned_binary()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let (handshake_tx, handshake_rx) = tokio::sync::mpsc::channel(8);
    let (release_tx, release_rx) = tokio::sync::watch::channel(server::Release::Held);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let observer = server::observe(listener, &directory, handshake_tx, release_rx, stop_rx);
    let workflows =
        native_scenario::exercise(&directory, &binary, &origin, handshake_rx, release_tx);
    let driver = async {
        let result = workflows.await;
        let stopped = stop_tx
            .send(())
            .map_err(|()| anyhow::anyhow!("observer stop receiver lost"));
        match (result, stopped) {
            (Ok(facts), Ok(())) => Ok(facts),
            (result, stopped) => Err(anyhow::anyhow!(
                "native workflow={result:?}; stop={stopped:?}"
            )),
        }
    };
    let (facts, requests) = tokio::join!(driver, observer);
    let facts = facts?;
    let requests = requests?;
    let measurements = ledger::qualify(&requests)?;
    let node_identity = certificate::identity(&binary)?;
    let endpoint_identity = certificate::identity(&std::env::current_exe()?)?;
    let workload = facts
        .get("workload")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("workload field missing"))?;
    native_certificate::write(
        &directory,
        "origin-budget-native.json",
        &serde_json::json!({
            "schema":1,
            "scenario":"native-origin-budget",
            "scope":"independent native Restate acquisition workflows on one host sharing the production working-directory origin lock across three pinned nodes and their serving endpoint children",
            "origin":origin,
            "configuration":{"delay_ms":DELAY_MS,"clock_tolerance_ms":CLOCK_TOLERANCE_MS,
                "inflight_budget":INFLIGHT_BUDGET,"burst_budget":1,"rival_endpoints":2,
                "origin_lock_root":"var/locks","server_connection_bound":8,"request_deadline_seconds":180},
            "identities":{"restate_node":node_identity,"endpoint_child":endpoint_identity},
            "facts":workload,
            "outcomes":facts,
            "requests":requests,
            "measurements":measurements,
            "http_ledger_sha256":certificate::digest(&std::fs::read(directory.join("http-ledger.jsonl"))?),
        }),
    )?;
    Ok(())
}

#[tokio::test]
#[ignore = "child-mode endpoint host; spawned by the native scenario with its fixture environment"]
async fn native_budget_endpoint_child() -> anyhow::Result<()> {
    fixture::serve().await
}

const DELAY_MS: u64 = 1000;
const CLOCK_TOLERANCE_MS: u64 = 50;
const INFLIGHT_BUDGET: usize = 1;
const BODY: &str = "catalog08 physical owner acquisition\n";
const OWNER_AGENT: &str = "CensusOriginBudgetQualification/owner";
const OWNER_START: &str = "/owner-start";
const TARGET: &str = "/owner-target";
const PROCESS_DEADLINE: Duration = Duration::from_secs(30);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_cli_workflows_do_not_multiply_the_physical_origin_budget() -> Result<()> {
    let directory = certificate::directory()?;
    let binary = process::binary()?;
    let identities = certificate::identities(&binary)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let (handshake_tx, handshake_rx) = tokio::sync::mpsc::channel(8);
    let (release_tx, release_rx) = tokio::sync::watch::channel(server::Release::Held);
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let observer = server::observe(listener, &directory, handshake_tx, release_rx, stop_rx);
    let workflows = scenario::exercise(&directory, &binary, &origin, handshake_rx, release_tx);
    let driver = async {
        let result = workflows.await;
        let stopped = stop_tx
            .send(())
            .map_err(|()| anyhow::anyhow!("observer stop receiver lost"));
        match (result, stopped) {
            (Ok(facts), Ok(())) => Ok(facts),
            (result, stopped) => Err(anyhow::anyhow!("workflow={result:?}; stop={stopped:?}")),
        }
    };
    let (facts, requests) = tokio::join!(driver, observer);
    let facts = facts?;
    let requests = requests?;
    let measurements = ledger::qualify(&requests)?;
    anyhow::ensure!(
        identities == certificate::identities(&binary)?,
        "executables changed during qualification"
    );
    certificate::publish(
        &directory,
        &origin,
        identities,
        facts,
        requests,
        measurements,
    )?;
    Ok(())
}
