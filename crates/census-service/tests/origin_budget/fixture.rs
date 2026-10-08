use anyhow::{ensure, Result};
use census_crawl::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_store::Store;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Input {
    pub origin: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome")]
pub(super) enum Reply {
    Complete { invocation_id: String, pid: u32, captures: Vec<FetchOutcomeWire> },
    Refused { invocation_id: String, pid: u32, origin: String, holder: serde_json::Value },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct FetchOutcomeWire {
    pub url: String,
    pub status: u16,
    pub content_digest: String,
    pub fetched_at: String,
    pub from_cache: bool,
    pub bytes: usize,
}

impl From<FetchOutcome> for FetchOutcomeWire {
    fn from(value: FetchOutcome) -> Self {
        Self { url: value.url, status: value.status, content_digest: value.content_digest,
            fetched_at: value.fetched_at, from_cache: value.from_cache, bytes: value.bytes }
    }
}

#[derive(Default)]
struct Counters {
    accepted: AtomicUsize,
    completed: AtomicUsize,
    active: AtomicUsize,
}

struct BudgetProbe {
    store: Arc<Store>,
    fetcher: Fetcher,
    counters: Arc<Counters>,
}

#[service(journal_retention = "1 hour", idempotency_retention = "1 hour")]
impl BudgetProbe {
    #[handler]
    async fn acquire(&self, ctx: Context<'_>, Json(input): Json<Input>) -> std::result::Result<Json<Reply>, HandlerError> {
        self.counters.accepted.fetch_add(1, Ordering::SeqCst);
        self.counters.active.fetch_add(1, Ordering::SeqCst);
        let result = self.acquire_pages(&ctx, input).await;
        self.counters.active.fetch_sub(1, Ordering::SeqCst);
        self.counters.completed.fetch_add(1, Ordering::SeqCst);
        result.map(Json)
    }
}

impl BudgetProbe {
    async fn acquire_pages(&self, ctx: &Context<'_>, input: Input) -> std::result::Result<Reply, HandlerError> {
        validate(&input)?;
        let mut captures = Vec::with_capacity(input.paths.len());
        for path in &input.paths {
            let reply = ctx.run(|| self.fetch_page(ctx.invocation_id(), &input.origin, path)).await?.into_inner();
            match reply {
                Reply::Complete { captures: page, .. } => captures.extend(page),
                refused @ Reply::Refused { .. } => return Ok(refused),
            }
        }
        Ok(Reply::Complete { invocation_id: ctx.invocation_id().to_string(), pid: std::process::id(), captures })
    }

    async fn fetch_page(&self, invocation: &str, origin: &str, path: &str) -> std::result::Result<Json<Reply>, HandlerError> {
        let result = self.fetcher.get(&format!("{origin}{path}"), &FetchOptions::default()).await;
        let reply = match result {
            Ok(capture) => Reply::Complete { invocation_id: invocation.to_string(), pid: std::process::id(), captures: vec![capture.into()] },
            Err(FetchError::OriginHeld { origin, holder }) => Reply::Refused {
                invocation_id: invocation.to_string(), pid: std::process::id(), origin,
                holder: serde_json::from_str(&holder).map_err(|error| TerminalError::new(error.to_string()))?,
            },
            Err(error) => return Err(TerminalError::new(error.to_string()).into()),
        };
        self.store.journal_done("private_origin_budget", &format!("{invocation}:{path}"), &reply)?;
        Ok(Json(reply))
    }
}

fn validate(input: &Input) -> std::result::Result<(), TerminalError> {
    let url = url::Url::parse(&input.origin).map_err(|error| TerminalError::new(error.to_string()))?;
    if url.scheme() != "http" || url.host_str() != Some("127.0.0.1") || url.port().is_none()
        || url.path() != "/" || input.paths.is_empty() || input.paths.len() > 6
        || input.paths.iter().any(|path| !path.starts_with('/') || path.contains("..") || path.contains('?')) {
        return Err(TerminalError::new("invalid private budget fixture input"));
    }
    Ok(())
}

pub(super) async fn serve() -> Result<()> {
    let root = PathBuf::from(std::env::var_os("CENSUS_BUDGET_FIXTURE_STORE").ok_or_else(|| anyhow::anyhow!("fixture store missing"))?);
    let listen = std::env::var("CENSUS_BUDGET_FIXTURE_LISTEN")?;
    let agent = std::env::var("CENSUS_BUDGET_FIXTURE_AGENT")?;
    let store = Arc::new(Store::open(&root)?);
    let fetcher = Fetcher::new(store.http_cache_dir(), Some(agent), Duration::from_millis(super::DELAY_MS),
        HashMap::new(), vec!["127.0.0.1".to_string()])?
        .with_origin_locks(census_service::census::DEFAULT_ORIGIN_LOCK_ROOT);
    let counters = Arc::new(Counters::default());
    let endpoint = Endpoint::builder().bind(BudgetProbe { store: Arc::clone(&store), fetcher, counters: Arc::clone(&counters) }).build();
    let listener = tokio::net::TcpListener::bind(&listen).await?;
    let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    HttpServer::new(endpoint).serve_with_cancel(listener, signal.recv()).await;
    store.flush()?;
    let accepted = counters.accepted.load(Ordering::SeqCst);
    let completed = counters.completed.load(Ordering::SeqCst);
    let active = counters.active.load(Ordering::SeqCst);
    let drain = serde_json::json!({"pid":std::process::id(),"accepted":accepted,"completed":completed,"remaining":active});
    std::fs::write(root.join("fixture-drain.json"), serde_json::to_vec_pretty(&drain)?)?;
    ensure!(active == 0 && accepted == completed, "fixture did not drain: {drain}");
    Ok(())
}
