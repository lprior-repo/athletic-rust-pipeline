use anyhow::{ensure, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;

use super::ledger::{self, Event, Request};
use super::{BODY, OWNER_AGENT, OWNER_START, TARGET};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Release {
    Held,
    Released,
}

#[derive(Debug)]
pub(super) struct Handshake {
    pub connection: usize,
    pub path: String,
    pub user_agent: String,
}

#[derive(Clone, Copy)]
pub(super) struct Bounds {
    pub connections: usize,
    pub events: usize,
}

pub(super) async fn observe(
    listener: TcpListener,
    directory: &Path,
    handshake: mpsc::Sender<Handshake>,
    release: watch::Receiver<Release>,
    stop: oneshot::Receiver<()>,
) -> Result<Vec<Request>> {
    observe_bounded(
        listener,
        directory,
        handshake,
        release,
        stop,
        Bounds {
            connections: 8,
            events: 16,
        },
    )
    .await
}

pub(super) async fn observe_bounded(
    listener: TcpListener,
    directory: &Path,
    handshake: mpsc::Sender<Handshake>,
    release: watch::Receiver<Release>,
    mut stop: oneshot::Receiver<()>,
    bounds: Bounds,
) -> Result<Vec<Request>> {
    let mut log = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(directory.join("http-ledger.jsonl"))?;
    let (event_tx, mut event_rx) = mpsc::channel(32);
    let mut tasks = JoinSet::new();
    let mut events = Vec::with_capacity(bounds.events);
    let mut connections = 0_usize;
    let epoch = Instant::now();
    let deadline = tokio::time::sleep(Duration::from_secs(120));
    tokio::pin!(deadline);
    let mut failure = None;
    for _ in 0..512 {
        let step: Result<bool> = tokio::select! {
            biased;
            _ = &mut deadline => Err(anyhow::anyhow!("physical observer exceeded 120 seconds")),
            event = event_rx.recv() => record(event, &mut log, &mut events, bounds.events).map(|()| false),
            joined = tasks.join_next(), if !tasks.is_empty() => joined_result(joined).map(|()| false),
            accepted = listener.accept() => match accepted {
                Ok((socket, _)) if connections < bounds.connections => {
                    connections = connections.saturating_add(1);
                    tasks.spawn(handle(socket, connections, epoch, event_tx.clone(), handshake.clone(), release.clone()));
                    Ok(false)
                }
                Ok((socket, _)) => { drop(socket); Err(anyhow::anyhow!("physical connection bound exceeded")) }
                Err(error) => Err(error.into()),
            },
            stopped = &mut stop => stopped.map(|()| true).map_err(anyhow::Error::from),
        };
        match step {
            Ok(false) => continue,
            Ok(true) => break,
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    drop(listener);
    drop(event_tx);
    let drained = drain(&mut tasks, &mut event_rx, &mut log, &mut events, bounds).await;
    log.sync_all()?;
    ensure!(
        connections <= bounds.connections,
        "physical connection bound exceeded"
    );
    drained?;
    if let Some(error) = failure {
        return Err(error);
    }
    ledger::assemble(events)
}

fn record(
    event: Option<Event>,
    log: &mut std::fs::File,
    events: &mut Vec<Event>,
    bound: usize,
) -> Result<()> {
    let event = event.ok_or_else(|| anyhow::anyhow!("physical ledger channel closed"))?;
    ensure!(events.len() < bound, "physical event bound exceeded");
    serde_json::to_writer(&mut *log, &event)?;
    log.write_all(b"\n")?;
    log.flush()?;
    events.push(event);
    Ok(())
}

fn joined_result(
    joined: Option<std::result::Result<Result<()>, tokio::task::JoinError>>,
) -> Result<()> {
    match joined {
        Some(Ok(result)) => result,
        Some(Err(error)) => Err(anyhow::anyhow!(
            "HTTP request task panicked/cancelled: {error}"
        )),
        None => Err(anyhow::anyhow!("HTTP request task disappeared")),
    }
}

async fn drain(
    tasks: &mut JoinSet<Result<()>>,
    events_rx: &mut mpsc::Receiver<Event>,
    log: &mut std::fs::File,
    events: &mut Vec<Event>,
    bounds: Bounds,
) -> Result<()> {
    let mut failures = Vec::new();
    let complete = tokio::time::timeout(Duration::from_secs(5), async {
        for _ in 0..bounds.connections {
            if tasks.is_empty() {
                break;
            }
            if let Err(error) = joined_result(tasks.join_next().await) {
                failures.push(error.to_string());
            }
        }
    })
    .await;
    if complete.is_err() {
        tasks.abort_all();
        for _ in 0..bounds.connections {
            if tasks.is_empty() {
                break;
            }
            match tokio::time::timeout(Duration::from_secs(5), tasks.join_next()).await {
                Ok(Some(Err(error))) if error.is_cancelled() => {
                    failures.push(format!("HTTP cleanup aborted task: {error}"))
                }
                Ok(joined) => {
                    if let Err(error) = joined_result(joined) {
                        failures.push(error.to_string());
                    }
                }
                Err(error) => failures.push(format!("HTTP aborted task reap timed out: {error}")),
            }
        }
        failures.push("HTTP drain timed out".to_string());
    }
    for _ in 0..bounds.events {
        match events_rx.try_recv() {
            Ok(event) => {
                if let Err(error) = record(Some(event), log, events, bounds.events) {
                    failures.push(error.to_string());
                }
            }
            Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected) => {
                break
            }
        }
    }
    ensure!(
        tasks.is_empty() && failures.is_empty(),
        "HTTP cleanup failures: {failures:?}; remaining={}",
        tasks.len()
    );
    Ok(())
}

async fn handle(
    mut socket: TcpStream,
    connection: usize,
    epoch: Instant,
    events: mpsc::Sender<Event>,
    handshake: mpsc::Sender<Handshake>,
    release: watch::Receiver<Release>,
) -> Result<()> {
    let result = tokio::time::timeout(
        Duration::from_secs(90),
        respond(&mut socket, connection, epoch, events, handshake, release),
    )
    .await;
    let shutdown = tokio::time::timeout(Duration::from_secs(5), socket.shutdown()).await;
    match (result, shutdown) {
        (Ok(Ok(())), Ok(Ok(()))) => Ok(()),
        (result, shutdown) => Err(anyhow::anyhow!(
            "HTTP request={result:?}; socket shutdown={shutdown:?}"
        )),
    }
}

async fn respond(
    socket: &mut TcpStream,
    connection: usize,
    epoch: Instant,
    events: mpsc::Sender<Event>,
    handshake: mpsc::Sender<Handshake>,
    mut release: watch::Receiver<Release>,
) -> Result<()> {
    let request = headers(socket).await?;
    let (method, path, user_agent) = parse(&request)?;
    let start = Event::Start {
        connection,
        at_ns: u64::try_from(epoch.elapsed().as_nanos())?,
        method: method.to_string(),
        path: path.to_string(),
        user_agent: user_agent.to_string(),
    };
    events.send(start).await?;
    if path == TARGET && user_agent == OWNER_AGENT {
        handshake
            .send(Handshake {
                connection,
                path: path.to_string(),
                user_agent: user_agent.to_string(),
            })
            .await?;
        release
            .wait_for(|state| *state == Release::Released)
            .await?;
    }
    let (status, response) = if path == OWNER_START {
        (
            302,
            format!(
                "HTTP/1.1 302 Found\r\nLocation: {TARGET}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
        )
    } else {
        (
            200,
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{BODY}",
                BODY.len()
            ),
        )
    };
    socket.write_all(response.as_bytes()).await?;
    events
        .send(Event::End {
            connection,
            at_ns: u64::try_from(epoch.elapsed().as_nanos())?,
            response_status: status,
        })
        .await?;
    Ok(())
}

async fn headers(socket: &mut TcpStream) -> Result<String> {
    let mut bytes = [0_u8; 8192];
    let mut size = 0;
    for _ in 0..8192 {
        let available = bytes
            .get_mut(size..)
            .ok_or_else(|| anyhow::anyhow!("HTTP header offset"))?;
        ensure!(!available.is_empty(), "HTTP header bound exceeded");
        let count = socket.read(available).await?;
        ensure!(count > 0, "HTTP connection closed before headers");
        size = size
            .checked_add(count)
            .ok_or_else(|| anyhow::anyhow!("HTTP header size overflow"))?;
        let request = bytes
            .get(..size)
            .ok_or_else(|| anyhow::anyhow!("HTTP header length"))?;
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(std::str::from_utf8(request)?.to_string());
        }
    }
    Err(anyhow::anyhow!("HTTP header read bound exceeded"))
}

fn parse(request: &str) -> Result<(&str, &str, &str)> {
    let mut lines = request.split("\r\n");
    let mut fields = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("HTTP request line missing"))?
        .split_whitespace();
    let method = fields
        .next()
        .ok_or_else(|| anyhow::anyhow!("HTTP method missing"))?;
    let path = fields
        .next()
        .ok_or_else(|| anyhow::anyhow!("HTTP path missing"))?;
    ensure!(
        fields.next() == Some("HTTP/1.1") && fields.next().is_none(),
        "unexpected HTTP protocol"
    );
    let mut agent = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("malformed HTTP header"))?;
        if name.eq_ignore_ascii_case("user-agent") {
            ensure!(agent.is_none(), "duplicate user-agent header");
            agent = Some(value.trim());
        }
    }
    Ok((
        method,
        path,
        agent.ok_or_else(|| anyhow::anyhow!("physical user-agent missing"))?,
    ))
}
