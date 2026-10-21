use super::TestResult;
use crate::spawn::{Spawner, TaskReport};
use restate_sdk::prelude::{Endpoint, HttpServer};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::oneshot;

pub(super) struct EndpointOwner {
    stop: Option<oneshot::Sender<()>>,
    region: Spawner,
    drained: Option<TaskReport>,
    pub(super) address: SocketAddr,
}

impl EndpointOwner {
    pub(super) async fn start(address: SocketAddr, endpoint: Endpoint) -> TestResult<Self> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        let address = listener.local_addr()?;
        let region = Spawner::with_capacity(1);
        let (stop, receiver) = oneshot::channel();
        region.spawn(async move {
            HttpServer::new(endpoint)
                .serve_with_cancel(listener, async move {
                    match receiver.await {
                        Ok(()) | Err(_) => {}
                    }
                })
                .await;
        })?;
        Ok(Self {
            stop: Some(stop),
            region,
            drained: None,
            address,
        })
    }

    pub(super) async fn stop(&mut self) -> TestResult<TaskReport> {
        if let Some(report) = self.drained {
            return Ok(report);
        }
        if let Some(stop) = self.stop.take() {
            stop.send(())
                .map_err(|_| "native endpoint lost its shutdown owner")?;
        }
        let report = self.region.drain(Duration::from_secs(15)).await?;
        self.drained = Some(report);
        check!(eq; (report.accepted, report.completed), (1, 1));
        check!(eq; report.disjoint_total(), report.accepted);
        check!(eq; (report.cancelled, report.aborted, report.panicked), (0, 0, 0));
        check!(eq; (report.failed, report.still_running, report.timed_out, report.remaining), (0, 0, 0, 0));
        Ok(report)
    }
}
