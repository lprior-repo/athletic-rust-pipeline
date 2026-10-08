#![forbid(unsafe_code)]

#[path = "origin_budget/certificate.rs"]
mod certificate;
#[path = "origin_budget/ledger.rs"]
mod ledger;
#[path = "origin_budget/process.rs"]
mod process;
#[path = "origin_budget/scenario.rs"]
mod scenario;
#[path = "origin_budget/server.rs"]
mod server;

use anyhow::Result;
use std::time::Duration;

const DELAY_MS: u64 = 1000;
const CLOCK_TOLERANCE_MS: u64 = 50;
const INFLIGHT_BUDGET: usize = 1;
const BODY: &str = "catalog08 physical owner acquisition\n";
const OWNER_AGENT: &str = "CensusOriginBudgetQualification/owner";
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
