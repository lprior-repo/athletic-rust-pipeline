use anyhow::{ensure, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

use super::fixture::{Input, Reply};
use super::native_api::{self, Submission};
use super::native_process::{self, Owned};
use super::server::{Handshake, Release};

#[derive(Debug, Serialize)]
pub(super) struct Call {
    pub endpoint: String,
    pub key: String,
    pub input: Input,
    pub original: Submission,
    pub output: Reply,
}

#[derive(Debug, Serialize)]
pub(super) struct Site {
    pub name: String,
    pub endpoint_index: usize,
    pub node_index: usize,
    pub endpoint_port: u16,
    pub ingress: String,
    pub admin: String,
    pub ready: Value,
    pub deployment: Value,
    pub journal: Value,
    pub drain: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct Facts {
    pub sites: Vec<Site>,
    pub calls: Vec<Call>,
    pub reached_connection: usize,
    pub holder: Value,
    pub retained_lock: PathBuf,
}

pub(super) async fn exercise(
    directory: &Path,
    binary: &Path,
    origin: &str,
    handshake: mpsc::Receiver<Handshake>,
    release: watch::Sender<Release>,
) -> Result<Value> {
    let mut children = Vec::with_capacity(6);
    let result = tokio::time::timeout(
        Duration::from_secs(180),
        drive(
            directory,
            binary,
            origin,
            &mut children,
            handshake,
            &release,
        ),
    )
    .await;
    release.send_replace(Release::Released);
    let cleanup = cleanup(&mut children).await;
    let snapshot = json!({
        "scenario":format!("{result:?}"),"cleanup":format!("{cleanup:?}"),"children":children,
    });
    super::native_certificate::write(directory, "native-process-outcomes.json", &snapshot)?;
    cleanup?;
    let mut facts = result??;
    collect_drains(&mut facts.sites, &children, &facts.calls)?;
    ensure!(children.len() == 6, "native process census is incomplete");
    Ok(json!({"workload":facts,"children":children,"children_reaped":children.len()}))
}

async fn drive(
    directory: &Path,
    binary: &Path,
    origin: &str,
    children: &mut Vec<Owned>,
    handshake: mpsc::Receiver<Handshake>,
    release: &watch::Sender<Release>,
) -> Result<Facts> {
    let client = native_api::client()?;
    let mut sites = launch(directory, binary, children)?;
    for site in &mut sites {
        site.ready = ready(directory, site, children).await?;
        site.deployment = native_api::register(
            &client,
            &site.admin,
            site.endpoint_port,
            &directory.join(&site.name),
        )
        .await?;
    }
    let mut facts = super::native_workload::contend(
        directory, origin, &client, sites, children, handshake, release,
    )
    .await?;
    for site in &mut facts.sites {
        let expected: Vec<_> = facts
            .calls
            .iter()
            .filter(|call| call.endpoint == site.name)
            .map(|call| &call.original)
            .collect();
        site.journal = native_api::journal(
            &client,
            &site.admin,
            &directory.join(format!("{}-invocations.json", site.name)),
            &expected,
        )
        .await?;
    }
    super::native_certificate::write(
        directory,
        "native-workload-outcomes.json",
        &serde_json::to_value(&facts)?,
    )?;
    Ok(facts)
}

fn launch(directory: &Path, binary: &Path, children: &mut Vec<Owned>) -> Result<Vec<Site>> {
    let reservations = native_process::reserve(12)?;
    let ports: Vec<_> = reservations
        .iter()
        .map(|socket| socket.local_addr().map(|addr| addr.port()))
        .collect::<std::io::Result<_>>()?;
    drop(reservations);
    let mut sites = Vec::with_capacity(3);
    for (name, ports) in ["owner", "rival-one", "rival-two"]
        .into_iter()
        .zip(ports.chunks_exact(4))
    {
        let endpoint_port = *ports
            .first()
            .ok_or_else(|| anyhow::anyhow!("endpoint port missing"))?;
        let endpoint_index = children.len();
        children.push(Owned::endpoint(directory, name, endpoint_port)?);
        let node_index = children.len();
        let node_ports = ports
            .get(1..)
            .ok_or_else(|| anyhow::anyhow!("node ports missing"))?;
        children.push(native_process::node(directory, name, binary, node_ports)?);
        let ingress = *node_ports
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("ingress port missing"))?;
        let admin = *node_ports
            .get(2)
            .ok_or_else(|| anyhow::anyhow!("admin port missing"))?;
        sites.push(Site {
            name: name.to_string(),
            endpoint_index,
            node_index,
            endpoint_port,
            ingress: format!("http://127.0.0.1:{ingress}"),
            admin: format!("http://127.0.0.1:{admin}"),
            ready: Value::Null,
            deployment: Value::Null,
            journal: Value::Null,
            drain: Value::Null,
        });
    }
    ensure!(sites.len() == 3, "native private site census incomplete");
    Ok(sites)
}

async fn ready(directory: &Path, site: &Site, children: &mut [Owned]) -> Result<Value> {
    let index = site.endpoint_index;
    let ready_path = child(children, index)?.root.join("fixture-ready.json");
    let mut ticks = tokio::time::interval(Duration::from_millis(25));
    for _ in 0..2400 {
        child(children, index)?.require_live()?;
        child(children, site.node_index)?.require_live()?;
        if ready_path.is_file() {
            let value: Value = serde_json::from_slice(&std::fs::read(&ready_path)?)?;
            let expected_lock = std::fs::canonicalize(
                directory.join(census_service::census::DEFAULT_ORIGIN_LOCK_ROOT),
            )?;
            ensure!(
                value.get("pid").and_then(Value::as_u64)
                    == Some(u64::from(child(children, index)?.pid))
                    && value.get("listen").and_then(Value::as_str)
                        == Some(format!("127.0.0.1:{}", site.endpoint_port).as_str())
                    && value.get("store")
                        == Some(&json!(std::fs::canonicalize(
                            &child(children, index)?.root
                        )?))
                    && value.get("origin_lock_root") == Some(&json!(expected_lock)),
                "foreign private endpoint ready identity: {value}"
            );
            return Ok(value);
        }
        ticks.tick().await;
    }
    Err(anyhow::anyhow!(
        "owned endpoint not ready: {}",
        ready_path.display()
    ))
}

pub(super) fn child(children: &mut [Owned], index: usize) -> Result<&mut Owned> {
    children
        .get_mut(index)
        .ok_or_else(|| anyhow::anyhow!("owned native child {index} missing"))
}

async fn cleanup(children: &mut [Owned]) -> Result<()> {
    let mut failures = Vec::new();
    for parity in [0, 1] {
        for (index, child) in children.iter_mut().enumerate() {
            if index % 2 == parity {
                if let Err(error) = child.stop().await {
                    failures.push(error.to_string());
                }
            }
        }
    }
    ensure!(
        children
            .iter()
            .all(|child| child.reaped && child.ports_released),
        "native cleanup left children/ports owned: {children:?}; {failures:?}"
    );
    ensure!(failures.is_empty(), "native cleanup failed: {failures:?}");
    Ok(())
}

fn collect_drains(sites: &mut [Site], children: &[Owned], calls: &[Call]) -> Result<()> {
    for site in sites {
        let endpoint = children
            .get(site.endpoint_index)
            .ok_or_else(|| anyhow::anyhow!("endpoint child absent"))?;
        let drain: Value =
            serde_json::from_slice(&std::fs::read(endpoint.root.join("fixture-drain.json"))?)?;
        let count = u64::try_from(
            calls
                .iter()
                .filter(|call| call.endpoint == site.name)
                .count(),
        )?;
        ensure!(
            drain.get("pid").and_then(Value::as_u64) == Some(u64::from(endpoint.pid))
                && drain.get("accepted").and_then(Value::as_u64) == Some(count)
                && drain.get("completed").and_then(Value::as_u64) == Some(count)
                && ["failed", "cancelled", "remaining"]
                    .iter()
                    .all(|field| drain.get(field).and_then(Value::as_u64) == Some(0)),
            "native endpoint outcomes do not balance original calls: {drain}"
        );
        site.drain = drain;
    }
    Ok(())
}
