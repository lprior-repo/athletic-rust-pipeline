use super::{
    note, BTreeMap, Child, Command, Duration, Instant, Path, Read, Stdio, DISCOVERY_ACCEPT,
    DISCOVER_ATTEMPTS, DISCOVER_RETRY_DELAY, SERVE_BIN,
};

pub(super) fn free_loopback_port() -> super::TestResult<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

pub(super) fn discovery_client() -> super::TestResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .http2_prior_knowledge()
        .build()?)
}

async fn discovery_once(client: &reqwest::Client, port: u16) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:{port}/discover");
    match client
        .get(&url)
        .header("accept", DISCOVERY_ACCEPT)
        .send()
        .await
    {
        Ok(response) if response.status() == reqwest::StatusCode::OK => response
            .json()
            .await
            .map_err(|error| format!("the manifest was not JSON: {error}")),
        Ok(response) => Err(format!("status {}", response.status())),
        Err(error) => Err(error.to_string()),
    }
}

pub(super) async fn endpoint_answers(client: &reqwest::Client, port: u16) -> bool {
    discovery_once(client, port).await.is_ok()
}

async fn poll_for_discovery(
    client: &reqwest::Client,
    port: u16,
    child: &mut Child,
) -> super::TestResult<serde_json::Value> {
    let mut last = String::from("<never attempted>");
    for _ in 0..DISCOVER_ATTEMPTS {
        match discovery_once(client, port).await {
            Ok(manifest) => return Ok(manifest),
            Err(reason) => last = reason,
        }
        if let Some(status) = child.try_wait()? {
            let mut stderr = String::new();
            if let Some(mut handle) = child.stderr.take() {
                handle.read_to_string(&mut stderr)?;
            }
            return Err(format!(
                "census-serve exited before answering /discover: {status:?}\nstderr:\n{stderr}"
            )
            .into());
        }
        tokio::time::sleep(DISCOVER_RETRY_DELAY).await;
    }
    Err(format!("census-serve never answered /discover on port {port}: last={last}").into())
}

pub(super) async fn spawn_serve(
    root: &Path,
    port: u16,
    drain_secs: u64,
) -> super::TestResult<(Child, serde_json::Value)> {
    let mut child = Command::new(SERVE_BIN)
        .arg("--data-dir")
        .arg(root)
        .arg("--listen")
        .arg(format!("127.0.0.1:{port}"))
        .arg("--drain-timeout")
        .arg(drain_secs.to_string())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let manifest = match poll_for_discovery(&discovery_client()?, port, &mut child).await {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    Ok((child, manifest))
}

fn manifest_services(manifest: &serde_json::Value) -> super::TestResult<Vec<String>> {
    manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .ok_or("discovery manifest carries no services array")?
        .iter()
        .map(|service| -> super::TestResult<String> {
            Ok(service
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("advertised service carries no name")?
                .to_string())
        })
        .collect()
}

pub(super) fn record_discovery_services(
    scenario: &str,
    manifest: &serde_json::Value,
) -> super::TestResult {
    let services = manifest_services(manifest)?;
    note(
        scenario,
        format!("discovery manifest services={services:?}"),
    );
    Ok(())
}

pub(super) fn wait_for_exit(
    child: &mut Child,
    bound: Duration,
) -> super::TestResult<Option<Duration>> {
    let started = Instant::now();
    while started.elapsed() < bound {
        if child.try_wait()?.is_some() {
            return Ok(Some(started.elapsed()));
        }
        std::thread::sleep(DISCOVER_RETRY_DELAY);
    }
    Ok(None)
}

pub(super) fn send_sigterm(child: &Child) -> super::TestResult {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()?;
    check!(status.success(), "kill -TERM was refused: {status}");
    Ok(())
}

pub(super) fn drain_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|line| line.starts_with("drained:"))
        .map_or("<no drain report>", |value| value)
}

pub(super) fn stop_reason_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|line| line.contains("stop_reason"))
        .map_or("<no stop reason>", |value| value)
}

pub(super) fn parse_drain_line(line: &str) -> BTreeMap<String, u64> {
    line.trim_start_matches("drained:")
        .split_whitespace()
        .filter_map(|field| field.split_once('='))
        .filter_map(|(name, value)| {
            value
                .parse::<u64>()
                .ok()
                .map(|value| (name.to_string(), value))
        })
        .collect()
}
