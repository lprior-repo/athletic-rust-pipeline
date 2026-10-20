use super::{key, replay, TeamsSourceRequest};
use crate::restate_services::jurisdiction::{JurisdictionCensus, TeamsSource};
use crate::restate_services::national::publication_tests::codec::{self, Frame};
use crate::restate_services::{CaptureExecution, Jobs};
use census_store::Store;
use restate_sdk::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct FixedSequenceClock {
    before: &'static str,
    after: &'static str,
    calls: AtomicUsize,
}

impl FixedSequenceClock {
    pub(super) fn new(before: &'static str, after: &'static str) -> Self {
        Self {
            before,
            after,
            calls: AtomicUsize::new(0),
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl census_store::clock::Clock for FixedSequenceClock {
    fn today(&self) -> String {
        "2026-10-14".into()
    }

    fn today_iso8601(&self) -> String {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.before.to_owned()
        } else {
            self.after.to_owned()
        }
    }

    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

#[derive(Default)]
pub(super) struct Events {
    pub(super) sleep: Option<Frame>,
    pub(super) failure: Option<String>,
    pub(super) output: Option<serde_json::Value>,
}

pub(super) struct Driver {
    pub(super) journal: Vec<Frame>,
    pub(super) run_values: Vec<serde_json::Value>,
    pub(super) decisions: Vec<serde_json::Value>,
}

impl Driver {
    pub(super) fn new(request: &TeamsSourceRequest) -> TestResult<Self> {
        Ok(Self {
            journal: vec![Frame::new(
                0x0400,
                codec::blob(14, &codec::value(&serde_json::to_vec(request)?)?)?,
            )],
            run_values: Vec::new(),
            decisions: Vec::new(),
        })
    }

    pub(super) async fn step(
        &mut self,
        store: &Arc<Store>,
        clock: &Arc<FixedSequenceClock>,
        request: &TeamsSourceRequest,
    ) -> TestResult<Events> {
        let frames = exchange(store, clock, request, &self.journal).await?;
        let mut events = Events::default();
        for frame in frames {
            match frame.kind {
                0x0411 => self.journal.push(frame),
                0x0005 => self.record_run(&frame)?,
                0x040C => {
                    check!(
                        events.sleep.is_none(),
                        "multiple sleep commands in one exchange"
                    );
                    self.journal.push(frame.clone());
                    events.sleep = Some(frame);
                }
                0x0401 => {
                    let value =
                        codec::field(frame.bytes(14)?, 1)?.ok_or("missing handler output value")?;
                    events.output = Some(serde_json::from_slice(value)?);
                }
                0x0002 => events.failure = Some(frame.text(2)?),
                0x0001 | 0x0003 => {}
                _ => return Err(format!("unexpected cooldown SDK frame {frame:?}").into()),
            }
        }
        check!(self.journal.len() < codec::MAX_FRAMES);
        Ok(events)
    }

    fn record_run(&mut self, frame: &Frame) -> TestResult {
        let value: serde_json::Value = serde_json::from_slice(frame.bytes(14)?)?;
        if value == "Proceed"
            || value.get("Sleep").is_some()
            || value.get("Blocked").is_some()
            || value.get("Settled").is_some()
        {
            self.decisions.push(value.clone());
        }
        self.run_values.push(value);
        self.journal.push(codec::notification(
            0x8011,
            frame.number(1)?,
            frame.bytes(14)?,
        )?);
        Ok(())
    }

    pub(super) fn complete_sleep(&mut self, sleep: &Frame) -> TestResult {
        check!(eq; sleep.kind, 0x040C);
        let mut completion = codec::integer(1, sleep.number(11)?);
        completion.extend(codec::blob(4, &[])?);
        self.journal.push(Frame::new(0x800C, completion));
        Ok(())
    }

    pub(super) fn persist(&self, path: &Path) -> TestResult {
        let mut bytes = Vec::new();
        for frame in &self.journal {
            bytes.extend(frame.encode()?);
        }
        check!(bytes.len() <= codec::MAX_WIRE_BYTES);
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub(super) fn restore(&mut self, path: &Path) -> TestResult {
        self.journal = codec::frames(&std::fs::read(path)?)?;
        Ok(())
    }

    pub(super) async fn until_sleep(
        &mut self,
        store: &Arc<Store>,
        clock: &Arc<FixedSequenceClock>,
        request: &TeamsSourceRequest,
    ) -> TestResult<Frame> {
        for _ in 0..8 {
            let events = self.step(store, clock, request).await?;
            check!(events.failure.is_none() && events.output.is_none());
            if let Some(sleep) = events.sleep {
                return Ok(sleep);
            }
        }
        Err("SDK emitted no sleep within eight resumptions".into())
    }

    pub(super) async fn until_failure(
        &mut self,
        store: &Arc<Store>,
        clock: &Arc<FixedSequenceClock>,
        request: &TeamsSourceRequest,
    ) -> TestResult<String> {
        for _ in 0..8 {
            let events = self.step(store, clock, request).await?;
            check!(events.sleep.is_none() && events.output.is_none());
            if let Some(failure) = events.failure {
                return Ok(failure);
            }
        }
        Err("SDK emitted no expected failure within eight resumptions".into())
    }

    pub(super) async fn until_output(
        &mut self,
        store: &Arc<Store>,
        clock: &Arc<FixedSequenceClock>,
        request: &TeamsSourceRequest,
    ) -> TestResult<serde_json::Value> {
        for _ in 0..8 {
            let events = self.step(store, clock, request).await?;
            check!(events.sleep.is_none() && events.failure.is_none());
            if let Some(output) = events.output {
                return Ok(output);
            }
        }
        Err("SDK emitted no expected output within eight resumptions".into())
    }
}

async fn exchange(
    store: &Arc<Store>,
    clock: &Arc<FixedSequenceClock>,
    request: &TeamsSourceRequest,
    journal: &[Frame],
) -> TestResult<Vec<Frame>> {
    let region = Arc::new(crate::spawn::Spawner::new());
    let owner = JurisdictionCensus::new(
        Arc::clone(store),
        clock.clone(),
        None,
        CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(store), Arc::clone(&region)),
    );
    let fetcher = Arc::new(
        census_crawl::net::Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            Default::default(),
            Vec::new(),
        )?
        .with_offline(true),
    );
    *owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&fetcher)));
    let source = TeamsSource::new(owner, Jobs::new(Arc::clone(store), Arc::clone(&region)));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!("http://{}/invoke/TeamsSource/run", listener.local_addr()?);
    let endpoint = Endpoint::builder().bind(source).build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async {
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .timeout(Duration::from_secs(5))
            .build()?;
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            replay::invoke(&client, &url, &key(request), journal),
        )
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    let drained = region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    result
}
