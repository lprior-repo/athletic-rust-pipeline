use anyhow::{ensure, Result};
use serde::Serialize;
use serde_json::Value;
use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};

use super::process::{Outcome, Workflow};
use super::server::{Handshake, Release};
use super::{BODY, OWNER_AGENT, OWNER_START, PROCESS_DEADLINE, TARGET};

#[derive(Debug, Serialize)]
pub(super) struct Facts {
    pub owner: Outcome,
    pub rivals: [Outcome; 2],
    pub replay: Outcome,
    pub reached_connection: usize,
    pub holder: Value,
    pub cache: Value,
    pub released_lock: PathBuf,
    pub children_reaped: usize,
}

pub(super) async fn exercise(
    directory: &Path,
    binary: &Path,
    origin: &str,
    handshake: mpsc::Receiver<Handshake>,
    release: watch::Sender<Release>,
) -> Result<Facts> {
    let mut workflows = Vec::with_capacity(4);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        contend(
            directory,
            binary,
            origin,
            &mut workflows,
            handshake,
            &release,
        ),
    )
    .await;
    release.send_replace(Release::Released);
    let mut failures = Vec::new();
    for workflow in &mut workflows {
        if let Err(error) = workflow.cleanup().await {
            failures.push(error.to_string());
        }
    }
    ensure!(
        failures.is_empty(),
        "owned process cleanup failed: {failures:?}; scenario={result:?}"
    );
    let mut facts = result??;
    ensure!(workflows.len() == 4, "workflow accounting incomplete");
    facts.children_reaped = workflows.len();
    Ok(facts)
}

async fn contend(
    directory: &Path,
    binary: &Path,
    origin: &str,
    workflows: &mut Vec<Workflow>,
    mut handshake: mpsc::Receiver<Handshake>,
    release: &watch::Sender<Release>,
) -> Result<Facts> {
    workflows.push(Workflow::spawn(
        directory,
        binary,
        origin,
        "owner",
        OWNER_START,
    )?);
    let reached = tokio::time::timeout(PROCESS_DEADLINE, handshake.recv())
        .await?
        .ok_or_else(|| anyhow::anyhow!("owner target handshake channel closed"))?;
    ensure!(
        reached.connection == 2 && reached.path == TARGET && reached.user_agent == OWNER_AGENT,
        "wrong reached physical handshake: {reached:?}"
    );
    workflow(workflows, 0)?.require_live()?;
    let lock_path = directory
        .join("var/locks")
        .join(census_crawl::net::origin_lock_file_name(origin));
    let holder: Value = serde_json::from_slice(&std::fs::read(&lock_path)?)?;
    ensure!(
        field(&holder, "pid")?.as_u64() == Some(u64::from(workflow(workflows, 0)?.pid()))
            && field(&holder, "origin")?.as_str() == Some(origin),
        "wrong reached lock holder: {holder}"
    );
    require_held(&lock_path)?;
    workflows.push(Workflow::spawn(
        directory,
        binary,
        origin,
        "rival-one",
        "/rival-one",
    )?);
    workflows.push(Workflow::spawn(
        directory,
        binary,
        origin,
        "rival-two",
        "/rival-two",
    )?);
    let rival_one = workflow(workflows, 1)?.finish().await?;
    let rival_two = workflow(workflows, 2)?.finish().await?;
    workflow(workflows, 0)?.require_live()?;
    let owner_pid = workflow(workflows, 0)?.pid();
    refusal(&rival_one, origin, owner_pid)?;
    refusal(&rival_two, origin, owner_pid)?;
    require_held(&lock_path)?;
    release.send(Release::Released)?;
    let owner = workflow(workflows, 0)?.finish().await?;
    successful(&owner, false)?;
    require_released(&lock_path)?;
    let cache = cache_facts(
        workflow(workflows, 0)?.store(),
        &format!("{origin}{OWNER_START}"),
        &format!("{origin}{TARGET}"),
    )?;
    let replay_child = Workflow::spawn_replay(directory, binary, workflow(workflows, 0)?)?;
    workflows.push(replay_child);
    let replay = workflow(workflows, 3)?.finish().await?;
    successful(&replay, true)?;
    ensure!(
        cache
            == cache_facts(
                workflow(workflows, 0)?.store(),
                &format!("{origin}{OWNER_START}"),
                &format!("{origin}{TARGET}")
            )?,
        "cache replay changed original acquisition facts"
    );
    let metadata = field(&cache, "metadata")?;
    let digest = field(metadata, "content_digest")?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("capture digest missing"))?;
    let fetched_at = field(metadata, "fetched_at")?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("capture timestamp missing"))?;
    for outcome in [&owner, &replay] {
        ensure!(outcome.stdout.contains(digest) && outcome.stdout.contains(&format!("fetched_at={fetched_at}")),
            "completed/cache outcome did not retain exact digest and capture timestamp: {outcome:?}");
    }
    Ok(Facts {
        owner,
        rivals: [rival_one, rival_two],
        replay,
        reached_connection: reached.connection,
        holder,
        cache,
        released_lock: lock_path,
        children_reaped: 0,
    })
}

fn workflow(workflows: &mut [Workflow], index: usize) -> Result<&mut Workflow> {
    workflows
        .get_mut(index)
        .ok_or_else(|| anyhow::anyhow!("owned workflow {index} absent"))
}

fn refusal(outcome: &Outcome, origin: &str, owner_pid: u32) -> Result<()> {
    ensure!(
        outcome.exit_code == 1,
        "rival did not exit with explicit failure: {outcome:?}"
    );
    let prefix =
        format!("Error: origin {origin} is held by another census-service process (holder: ");
    let holder = outcome
        .stderr
        .trim()
        .strip_prefix(&prefix)
        .and_then(|text| text.strip_suffix(')'))
        .ok_or_else(|| anyhow::anyhow!("not the typed OriginHeld CLI error: {outcome:?}"))?;
    let record: Value = serde_json::from_str(holder)?;
    ensure!(
        field(&record, "pid")?.as_u64() == Some(u64::from(owner_pid))
            && field(&record, "origin")?.as_str() == Some(origin),
        "OriginHeld named incorrect owner: {outcome:?}"
    );
    ensure!(
        !outcome.stdout.contains("source fetched"),
        "refusal reported silent fetch success: {outcome:?}"
    );
    Ok(())
}

fn successful(outcome: &Outcome, cached: bool) -> Result<()> {
    ensure!(
        outcome.exit_code == 0 && outcome.stderr.trim().is_empty(),
        "owner/replay failed: {outcome:?}"
    );
    ensure!(
        outcome.stdout.contains("source fetched")
            && outcome.stdout.contains("status=200")
            && outcome.stdout.contains(&format!("from_cache={cached}")),
        "wrong completed/cache outcome: {outcome:?}"
    );
    Ok(())
}

pub(super) fn require_held(path: &Path) -> Result<()> {
    let file = File::options().read(true).write(true).open(path)?;
    match file.try_lock() {
        Err(TryLockError::WouldBlock) => Ok(()),
        Ok(()) => {
            file.unlock()?;
            Err(anyhow::anyhow!(
                "reached owner did not hold exclusive origin lock"
            ))
        }
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

pub(super) fn require_released(path: &Path) -> Result<()> {
    let file = File::options().read(true).write(true).open(path)?;
    match file.try_lock() {
        Ok(()) => {
            file.unlock()?;
            Ok(())
        }
        Err(error) => Err(anyhow::anyhow!(
            "owner exited without releasing origin lock: {error:?}"
        )),
    }
}

pub(super) fn cache_facts(store: &Path, requested: &str, served: &str) -> Result<Value> {
    let mut captures = Vec::new();
    for (index, entry) in std::fs::read_dir(store.join("http"))?.enumerate() {
        ensure!(index < 64, "cache root exceeded fixture bound");
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("non-UTF8 cache name"))?;
        let Some(key) = name.strip_suffix(".meta.json") else {
            continue;
        };
        let encoded = std::fs::read(&path)?;
        let metadata: Value = serde_json::from_slice(&encoded)?;
        if field(&metadata, "url")?.as_str() != Some(requested) {
            continue;
        }
        let body_path = path.with_file_name(format!("{key}.body"));
        let body = std::fs::read(&body_path)?;
        ensure!(
            body == BODY.as_bytes(),
            "complete target body differs from physical 200 response"
        );
        ensure!(
            field(&metadata, "method")?.as_str() == Some("GET")
                && field(&metadata, "status")?.as_u64() == Some(200)
                && field(&metadata, "bytes")?.as_u64() == Some(u64::try_from(body.len())?)
                && field(&metadata, "content_digest")?.as_str()
                    == Some(super::certificate::digest(&body).as_str())
                && field(&metadata, "response_url")?.as_str() == Some(served)
                && field(&metadata, "content_type")?.as_str() == Some("text/plain"),
            "wrong complete capture metadata: {metadata}"
        );
        let fetched_at = field(&metadata, "fetched_at")?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("capture timestamp missing"))?;
        chrono::DateTime::parse_from_rfc3339(fetched_at)?;
        captures.push(serde_json::json!({"metadata":metadata,"metadata_sha256":super::certificate::digest(&encoded),
            "metadata_path":path,"body_path":body_path,"body_sha256":super::certificate::digest(&body)}));
    }
    ensure!(
        captures.len() == 1,
        "expected exactly one completed target capture: {captures:?}"
    );
    captures
        .pop()
        .ok_or_else(|| anyhow::anyhow!("target capture missing"))
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("required native fact {key} missing: {value}"))
}
