use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

use super::fixture::{Input, Reply};
use super::native_api;
use super::native_process::Owned;
use super::native_scenario::{Call, Facts, Site};
use super::server::{Handshake, Release};
use super::{OWNER_AGENT, TARGET};

pub(super) async fn contend(
    directory: &Path,
    origin: &str,
    client: &reqwest::Client,
    sites: Vec<Site>,
    children: &mut [Owned],
    mut handshake: mpsc::Receiver<Handshake>,
    release: &watch::Sender<Release>,
) -> Result<Facts> {
    let owner = site_by_name(&sites, "owner")?;
    let rival_one = site_by_name(&sites, "rival-one")?;
    let rival_two = site_by_name(&sites, "rival-two")?;
    for site in &sites {
        super::native_scenario::child(children, site.node_index)?.require_live()?;
        super::native_scenario::child(children, site.endpoint_index)?.require_live()?;
    }
    let mut calls = Vec::with_capacity(4);

    let owner_input = Input {
        origin: origin.to_string(),
        paths: vec![TARGET.to_string()],
    };
    let owner_key = "owner-acquire";
    let owner_submission = native_api::submit(
        client,
        &owner.ingress,
        owner_key,
        &owner_input,
        &directory.join("owner-submission.json"),
    )
    .await?;

    let reached = tokio::time::timeout(Duration::from_secs(30), handshake.recv())
        .await
        .map_err(|_| anyhow::anyhow!("owner target handshake timed out"))?
        .ok_or_else(|| anyhow::anyhow!("owner target handshake channel closed"))?;
    ensure!(
        reached.connection == 2 && reached.path == TARGET && reached.user_agent == OWNER_AGENT,
        "wrong reached physical handshake: {reached:?}"
    );

    let lock_path = directory
        .join("var/locks")
        .join(census_crawl::net::origin_lock_file_name(origin));
    let holder = holder_record(&lock_path)?;
    let holder_pid = holder
        .get("pid")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("lock holder pid missing"))?;
    ensure!(
        holder.get("origin").and_then(Value::as_str) == Some(origin),
        "lock holder origin mismatch: {holder}"
    );
    require_held(&lock_path)?;

    let rival_one_input = Input {
        origin: origin.to_string(),
        paths: vec!["/rival-one".to_string()],
    };
    let rival_one_key = "rival-one-acquire";
    let rival_one_submission = native_api::submit(
        client,
        &rival_one.ingress,
        rival_one_key,
        &rival_one_input,
        &directory.join("rival-one-submission.json"),
    )
    .await?;

    let rival_two_input = Input {
        origin: origin.to_string(),
        paths: vec!["/rival-two".to_string()],
    };
    let rival_two_key = "rival-two-acquire";
    let rival_two_submission = native_api::submit(
        client,
        &rival_two.ingress,
        rival_two_key,
        &rival_two_input,
        &directory.join("rival-two-submission.json"),
    )
    .await?;

    let rival_one_reply = native_api::attach(
        client,
        &rival_one.ingress,
        &rival_one_submission,
        &directory.join("rival-one-attach.json"),
    )
    .await?;
    assert_rival(&rival_one_reply, origin, holder_pid)?;

    let rival_two_reply = native_api::attach(
        client,
        &rival_two.ingress,
        &rival_two_submission,
        &directory.join("rival-two-attach.json"),
    )
    .await?;
    assert_rival(&rival_two_reply, origin, holder_pid)?;

    require_held(&lock_path)?;

    release.send_replace(Release::Released);

    let owner_reply = native_api::attach(
        client,
        &owner.ingress,
        &owner_submission,
        &directory.join("owner-attach.json"),
    )
    .await?;
    assert_owner(&owner_reply)?;

    let owner_captures = owner_captures(&owner_reply)?;
    let content_digest = owner_captures
        .first()
        .and_then(|c| c.get("content_digest"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("owner capture content_digest missing"))?
        .to_string();
    let fetched_at = owner_captures
        .first()
        .and_then(|c| c.get("fetched_at"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("owner capture fetched_at missing"))?
        .to_string();

    calls.push(Call {
        endpoint: "owner".to_string(),
        key: owner_key.to_string(),
        input: owner_input.clone(),
        original: owner_submission.clone(),
        output: owner_reply.clone(),
    });
    calls.push(Call {
        endpoint: "rival-one".to_string(),
        key: rival_one_key.to_string(),
        input: rival_one_input.clone(),
        original: rival_one_submission.clone(),
        output: rival_one_reply.clone(),
    });
    calls.push(Call {
        endpoint: "rival-two".to_string(),
        key: rival_two_key.to_string(),
        input: rival_two_input.clone(),
        original: rival_two_submission.clone(),
        output: rival_two_reply.clone(),
    });

    require_held(&lock_path)?;
    let retained = holder_record(&lock_path)?
        .get("pid")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("retained lock holder pid missing"))?;
    ensure!(
        retained == holder_pid,
        "origin lock ownership moved while the owner served: {retained} != {holder_pid}"
    );

    let replay_input = Input {
        origin: origin.to_string(),
        paths: vec![TARGET.to_string()],
    };
    let replay_key = "owner-replay";
    let replay_submission = native_api::submit(
        client,
        &owner.ingress,
        replay_key,
        &replay_input,
        &directory.join("replay-submission.json"),
    )
    .await?;

    let replay_reply = native_api::attach(
        client,
        &owner.ingress,
        &replay_submission,
        &directory.join("replay-attach.json"),
    )
    .await?;
    assert_replay(&replay_reply, &content_digest, &fetched_at)?;

    calls.push(Call {
        endpoint: "owner".to_string(),
        key: replay_key.to_string(),
        input: replay_input,
        original: replay_submission,
        output: replay_reply,
    });

    Ok(Facts {
        sites,
        calls,
        reached_connection: reached.connection,
        holder: json!({
            "pid": holder_pid,
            "origin": origin
        }),
        retained_lock: lock_path,
    })
}

fn site_by_name<'a>(sites: &'a [Site], name: &str) -> Result<&'a Site> {
    sites
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("site {name} missing"))
}

fn assert_owner(reply: &Reply) -> Result<()> {
    match reply {
        Reply::Complete { captures, .. } => {
            ensure!(!captures.is_empty(), "owner completed with no captures");
            ensure!(
                captures.iter().all(|c| c.status == 200 && !c.from_cache),
                "owner captures not all fresh 200: {captures:?}"
            );
            Ok(())
        }
        Reply::Refused { .. } => Err(anyhow::anyhow!("owner was refused")),
    }
}

fn assert_rival(reply: &Reply, origin: &str, expected_pid: u64) -> Result<()> {
    match reply {
        Reply::Refused {
            origin: held_origin,
            holder,
            ..
        } => {
            ensure!(held_origin == origin, "wrong refused origin: {held_origin}");
            let pid = holder
                .get("pid")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("refusal holder pid missing"))?;
            ensure!(
                pid == expected_pid,
                "refusal names wrong holder pid: {pid} != {expected_pid}"
            );
            Ok(())
        }
        Reply::Complete { .. } => Err(anyhow::anyhow!("rival completed instead of being refused")),
    }
}

fn owner_captures(reply: &Reply) -> Result<Vec<Value>> {
    match reply {
        Reply::Complete { captures, .. } => Ok(captures
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<Vec<_>, _>>()?),
        Reply::Refused { .. } => Err(anyhow::anyhow!("no captures on refused")),
    }
}

fn assert_replay(reply: &Reply, expected_digest: &str, expected_fetched_at: &str) -> Result<()> {
    match reply {
        Reply::Complete { captures, .. } => {
            ensure!(!captures.is_empty(), "replay completed with no captures");
            let first = &captures[0];
            ensure!(first.from_cache, "replay capture not from_cache: {first:?}");
            ensure!(
                first.content_digest == expected_digest,
                "replay digest changed: {} != {}",
                first.content_digest,
                expected_digest
            );
            ensure!(
                first.fetched_at == expected_fetched_at,
                "replay fetched_at changed: {} != {}",
                first.fetched_at,
                expected_fetched_at
            );
            Ok(())
        }
        Reply::Refused { .. } => Err(anyhow::anyhow!("replay was refused")),
    }
}

fn require_held(path: &Path) -> Result<()> {
    use std::fs::TryLockError;
    let file = File::open(path)?;
    match file.try_lock() {
        Err(TryLockError::WouldBlock) => Ok(()),
        Ok(()) => Err(anyhow::anyhow!("origin lock not held")),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

fn holder_record(path: &Path) -> Result<Value> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    ensure!(!bytes.is_empty(), "origin lock file is empty");
    Ok(serde_json::from_slice(&bytes)?)
}
