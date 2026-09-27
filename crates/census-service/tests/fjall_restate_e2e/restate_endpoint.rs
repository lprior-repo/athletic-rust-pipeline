use athleticnet_browser::BrowserSettings;
use census_service::bootstrap::{serve_until, DrainReport, ServeOptions, StopReason};
use census_store::{Store, Table};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinSet;
use url::Url;

const DISCOVERY_ACCEPT: &str = "application/vnd.restate.endpointmanifest.v4+json";
const DISCOVERY_ATTEMPTS: usize = 50;
const DISCOVERY_RETRY_DELAY: Duration = Duration::from_millis(100);
const EXPECTED_SERVICES: [&str; 9] = [
    "Census",
    "Consolidate",
    "Report",
    "Bests",
    "Workbook",
    "Ingest",
    "Sweep",
    "JurisdictionCensus",
    "NationalCensus",
];

fn free_local_address() -> SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("binding an ephemeral port");
    let address = listener.local_addr().expect("reading the bound address");
    drop(listener);
    address
}

async fn discover_manifest(client: &reqwest::Client, url: &str) -> serde_json::Value {
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
        panic!("{url} never answered within {DISCOVERY_ATTEMPTS} attempts: {last_error:?}");
    };
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "discovery must answer 200 while the endpoint is up"
    );
    response.json().await.unwrap()
}

fn service_names(manifest: &serde_json::Value) -> Vec<String> {
    manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .expect("the discovery manifest carries a services array")
        .iter()
        .filter_map(|service| service.get("name").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect()
}

async fn discover_service_names(client: &reqwest::Client, url: &str) -> Vec<String> {
    service_names(&discover_manifest(client, url).await)
}

fn discovery_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .http2_prior_knowledge()
        .build()
        .unwrap()
}

async fn drain_after_shutdown(
    tasks: &mut JoinSet<anyhow::Result<DrainReport>>,
    shutdown: tokio::sync::oneshot::Sender<()>,
) -> DrainReport {
    shutdown
        .send(())
        .expect("the supervisor still awaits the shutdown request");
    let outcome = tokio::time::timeout(Duration::from_secs(30), tasks.join_next()).await;
    let joined = match outcome {
        Ok(joined) => joined,
        Err(_) => {
            tasks.abort_all();
            panic!("serve_until did not return within 30s of the shutdown request");
        }
    };
    joined
        .expect("the serve task vanished without a drain report")
        .expect("the serve task panicked")
        .expect("serve_until returned an error")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restate_endpoint_advertises_services_and_drains_on_request() {
    let dir = tempfile::tempdir().unwrap();
    let listen = free_local_address();
    let data_dir = dir.path().join("data");
    let options = ServeOptions {
        listen,
        data_dir: data_dir.clone(),
        max_concurrent: 4,
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
    let client = discovery_client();
    let manifest = discover_manifest(&client, &url).await;
    let names = service_names(&manifest);
    for expected in EXPECTED_SERVICES {
        assert!(
            names.iter().any(|name| name == expected),
            "the discovery manifest {names:?} does not advertise {expected}"
        );
    }
    assert_eq!(
        names.len(),
        EXPECTED_SERVICES.len(),
        "the endpoint advertises exactly its services, nothing more: {names:?}"
    );

    let advertised = 60 * 60 * 1000;
    let services = manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .expect("the discovery manifest carries a services array");
    assert_eq!(
        services.len(),
        EXPECTED_SERVICES.len(),
        "every advertised service carries the timeouts: {services:?}"
    );
    for service in services {
        let name = service
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        assert_eq!(
            service
                .get("inactivityTimeout")
                .and_then(serde_json::Value::as_u64),
            Some(advertised),
            "{name} does not advertise the census inactivity timeout: {service:?}"
        );
        assert_eq!(
            service
                .get("abortTimeout")
                .and_then(serde_json::Value::as_u64),
            Some(advertised),
            "{name} does not advertise the census abort timeout: {service:?}"
        );
    }

    let report = drain_after_shutdown(&mut tasks, shutdown_tx).await;
    assert_eq!(report.stop_reason, StopReason::Requested);
    assert!(
        report.accepted >= 1,
        "no accepted work in the drain report: {report:?}"
    );
    assert_eq!(
        report.panicked, 0,
        "a task panicked during drain: {report:?}"
    );

    let reopened = Store::open(&data_dir).unwrap();
    assert_eq!(reopened.stats().unwrap().tables.len(), Table::ALL.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restate_endpoint_advertises_the_lane_when_it_serves_one() {
    let dir = tempfile::tempdir().unwrap();
    let listen = free_local_address();
    let options = ServeOptions {
        listen,
        data_dir: dir.path().join("data"),
        max_concurrent: 4,
        drain_timeout: Duration::from_secs(5),
        lane: Some(BrowserSettings {
            cdp_endpoint: None,
            executable: PathBuf::from("/nonexistent/chromium"),
            profile_dir: dir.path().join("profile"),
            source_origin: Url::parse("https://www.athletic.net/").unwrap(),
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
    let client = discovery_client();
    let names = discover_service_names(&client, &url).await;
    assert!(
        names.iter().any(|name| name == "BrowserSession"),
        "the discovery manifest {names:?} does not advertise the lane"
    );
    assert_eq!(
        names.len(),
        EXPECTED_SERVICES.len() + 1,
        "the endpoint advertises its services and the lane, nothing more: {names:?}"
    );

    let report = drain_after_shutdown(&mut tasks, shutdown_tx).await;
    assert_eq!(report.stop_reason, StopReason::Requested);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unrequested_stop_does_not_end_the_endpoint_at_the_drain_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let listen = free_local_address();
    let options = ServeOptions {
        listen,
        data_dir: dir.path().join("data"),
        max_concurrent: 4,
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
    let client = discovery_client();
    let names = discover_service_names(&client, &url).await;
    assert!(
        names.iter().any(|name| name == EXPECTED_SERVICES[0]),
        "the discovery manifest {names:?} does not advertise {}",
        EXPECTED_SERVICES[0]
    );

    let deadlines = 3;
    tokio::time::sleep(Duration::from_millis(250) * deadlines).await;
    let still_serving = discover_service_names(&client, &url).await;
    assert!(
        still_serving
            .iter()
            .any(|name| name == EXPECTED_SERVICES[0]),
        "the endpoint stopped answering after {} drain deadlines: {still_serving:?}",
        deadlines
    );
    assert!(
        tasks.try_join_next().is_none(),
        "the supervisor returned on its own, without a stop request"
    );

    let report = drain_after_shutdown(&mut tasks, shutdown_tx).await;
    assert_eq!(report.stop_reason, StopReason::Requested);
    assert_eq!(
        report.timed_out, 0,
        "a stop request must drain inside the deadline: {report:?}"
    );
}
