use crate::TestResult;
use athleticnet_browser::BrowserSettings;
use census_service::bootstrap::{
    serve_until, DrainReport, ServeOptions, StopReason, DEFAULT_MEMORY_BUDGET_BYTES,
};
use census_store::Store;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinSet;
use url::Url;

const DISCOVERY_ACCEPT: &str = "application/vnd.restate.endpointmanifest.v4+json";
const DISCOVERY_ATTEMPTS: usize = 50;
const DISCOVERY_RETRY_DELAY: Duration = Duration::from_millis(100);

fn free_local_address() -> TestResult<SocketAddr> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    drop(listener);
    Ok(address)
}

async fn discover_manifest(client: &reqwest::Client, url: &str) -> TestResult<serde_json::Value> {
    let mut answered = None;
    let mut last_error = None;
    for _ in 0..DISCOVERY_ATTEMPTS {
        match client
            .get(url)
            .header("accept", DISCOVERY_ACCEPT)
            .send()
            .await
        {
            Ok(response) => {
                answered = Some(response);
                break;
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(DISCOVERY_RETRY_DELAY).await;
            }
        }
    }
    let Some(response) = answered else {
        return Err(format!(
            "{url} never answered within {DISCOVERY_ATTEMPTS} attempts: {last_error:?}"
        )
        .into());
    };
    check!(eq; response.status(),
    reqwest::StatusCode::OK,
    "discovery must answer 200 while the endpoint is up");
    Ok(response.json().await?)
}

fn service_names(manifest: &serde_json::Value) -> TestResult<Vec<String>> {
    manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .ok_or("discovery manifest carries no services array")?
        .iter()
        .map(|service| -> TestResult<String> {
            Ok(service
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("advertised service carries no string name")?
                .to_owned())
        })
        .collect()
}

async fn discover_service_names(client: &reqwest::Client, url: &str) -> TestResult<Vec<String>> {
    service_names(&discover_manifest(client, url).await?)
}

fn discovery_client() -> TestResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .http2_prior_knowledge()
        .build()?)
}

async fn drain_after_shutdown(
    tasks: &mut JoinSet<anyhow::Result<DrainReport>>,
    shutdown: tokio::sync::oneshot::Sender<()>,
    listen: SocketAddr,
) -> TestResult<DrainReport> {
    shutdown
        .send(())
        .map_err(|_| "supervisor no longer awaits shutdown request")?;
    let outcome = tokio::time::timeout(Duration::from_secs(30), tasks.join_next()).await;
    let joined = match outcome {
        Ok(joined) => joined,
        Err(_) => {
            tasks.abort_all();
            return Err("serve_until did not return within 30s of shutdown request".into());
        }
    };
    let report = joined.ok_or("serve task vanished without drain report")???;
    check!(tasks.is_empty(), "the supervisor task was not reaped");
    check!(eq; report.remaining, 0, "work remains after drain: {report:?}");
    check!(eq; report.endpoint_shutdown,
    census_service::bootstrap::EndpointShutdown::Completed,
    "effect admission must close only after the endpoint stops: {report:?}");
    check!(eq; report.panicked, 0,
    "a task panicked during drain: {report:?}");
    check!(eq; report.accepted,
    report.completed + report.cancelled + report.aborted + report.panicked,
    "accepted work was lost or counted twice: {report:?}");
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(listen),
    )
    .await?;
    match connection {
        Err(error) => check!(eq; error.kind(),
        std::io::ErrorKind::ConnectionRefused,
        "the stopped endpoint must refuse new connections"),
        Ok(_) => return Err("endpoint still accepts connections after shutdown".into()),
    }
    Ok(report)
}

#[test]
fn restate_endpoint_advertises_services_and_drains_on_request() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let listen = free_local_address()?;
            let data_dir = dir.path().join("data");
            let options = ServeOptions {
                listen,
                data_dir: data_dir.clone(),
                max_concurrent: 4,
                memory_budget_bytes: DEFAULT_MEMORY_BUDGET_BYTES,
                drain_timeout: Duration::from_secs(5),
                lane: None,
            };

            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let mut tasks: JoinSet<anyhow::Result<DrainReport>> = JoinSet::new();
            tasks.spawn(async move {
                serve_until(options, async move {
                    let _ = shutdown_rx.await;
                })
                .await
            });

            let url = format!("http://{listen}/discover");
            let client = discovery_client()?;
            let manifest = discover_manifest(&client, &url).await?;
            let names = service_names(&manifest)?;
            check!(
                !names.iter().any(|name| name == "BrowserSession"),
                "the endpoint advertises a browser capability without a configured lane: {names:?}"
            );
            check!(
                tasks.try_join_next().is_none(),
                "the supervisor returned before a stop request"
            );

            let report = drain_after_shutdown(&mut tasks, shutdown_tx, listen).await?;
            check!(eq; report.stop_reason, StopReason::Requested);
            check!(
                report.accepted >= 1,
                "no accepted work in the drain report: {report:?}"
            );
            check!(eq; report.timed_out, 0, "requested drain timed out: {report:?}");
            check!(eq; report.aborted, 0,
    "requested drain aborted work: {report:?}");

            let reopened = Store::open(&data_dir)?;
            reopened.flush()?;
            Ok(())
        })
}

#[test]
fn restate_endpoint_advertises_the_lane_when_it_serves_one() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let listen = free_local_address()?;
            let options = ServeOptions {
                listen,
                data_dir: dir.path().join("data"),
                max_concurrent: 4,
                memory_budget_bytes: DEFAULT_MEMORY_BUDGET_BYTES,
                drain_timeout: Duration::from_secs(5),
                lane: Some(BrowserSettings {
                    cdp_endpoint: None,
                    executable: PathBuf::from("/nonexistent/chromium"),
                    profile_dir: dir.path().join("profile"),
                    source_origin: Url::parse("https://www.athletic.net/")?,
                    tabs: 1,
                    request_timeout: Duration::from_secs(30),
                    challenge_wait: Duration::from_secs(5),
                    headed: true,
                }),
            };

            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let mut tasks: JoinSet<anyhow::Result<DrainReport>> = JoinSet::new();
            tasks.spawn(async move {
                serve_until(options, async move {
                    let _ = shutdown_rx.await;
                })
                .await
            });

            let url = format!("http://{listen}/discover");
            let client = discovery_client()?;
            let names = discover_service_names(&client, &url).await?;
            check!(
                names.iter().any(|name| name == "BrowserSession"),
                "the discovery manifest {names:?} does not advertise the lane"
            );
            check!(
                tasks.try_join_next().is_none(),
                "the supervisor returned before a stop request"
            );

            let report = drain_after_shutdown(&mut tasks, shutdown_tx, listen).await?;
            check!(eq; report.stop_reason, StopReason::Requested);
            check!(eq; report.timed_out, 0,
    "requested lane drain timed out: {report:?}");
            check!(eq; report.aborted, 0,
    "requested lane drain aborted work: {report:?}");
            Ok(())
        })
}

#[test]
fn an_unrequested_stop_does_not_end_the_endpoint_at_the_drain_deadline() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let listen = free_local_address()?;
            let options = ServeOptions {
                listen,
                data_dir: dir.path().join("data"),
                max_concurrent: 4,
                memory_budget_bytes: DEFAULT_MEMORY_BUDGET_BYTES,
                drain_timeout: Duration::from_millis(250),
                lane: None,
            };

            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let mut tasks: JoinSet<anyhow::Result<DrainReport>> = JoinSet::new();
            tasks.spawn(async move {
                serve_until(options, async move {
                    let _ = shutdown_rx.await;
                })
                .await
            });

            let url = format!("http://{listen}/discover");
            let client = discovery_client()?;
            discover_service_names(&client, &url).await?;

            let deadlines = 3;
            tokio::time::sleep(Duration::from_millis(250) * deadlines).await;
            discover_service_names(&client, &url).await?;
            check!(
                tasks.try_join_next().is_none(),
                "the supervisor returned on its own, without a stop request"
            );

            let report = drain_after_shutdown(&mut tasks, shutdown_tx, listen).await?;
            check!(eq; report.stop_reason, StopReason::Requested);
            check!(eq; report.timed_out, 0,
    "a stop request must drain inside the deadline: {report:?}");
            Ok(())
        })
}
