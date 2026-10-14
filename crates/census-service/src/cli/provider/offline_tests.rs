use super::{build_provider_fetcher, dispatch, ProviderArgs};
use crate::cli::{Cli, Command};
use census_crawl::net::bridge::{Action, BrowserLane, RequestSpec};
use census_crawl::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_crawl::{AdapterContext, CaptureContext, CollectionDisposition};
use census_domain::model::{CanonicalPerformance, ExactSeconds, Mark, SchoolYear};
use census_store::{Store, Table};
use clap::Parser;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[path = "../../../tests/common/capture_cache.rs"]
mod capture_cache;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
const ACQUIRED_AT: &str = "2026-09-22T03:55:31Z";
const BROWSER_URL: &str = "https://www.athletic.net/athlete/123456/track-and-field";
const REQUEST_DEADLINE: Duration = Duration::from_secs(5);
const BROWSER_CAPTURE: &str = r#"{"outcome":"captured","response":{"status":200,"headers":[],"body":"b2JzZXJ2ZWQ=","rankings":null},"challenge":false,"retry_after_ms":null,"fetched_at_ms":1758542400000}"#;
const LIST: &[u8] =
    include_bytes!("../../../../census-crawl/tests/fixtures/tfrrs/indiana_list_5489_excerpt.html");

fn offline_cli(root: &Path) -> TestResult<Cli> {
    Ok(Cli::try_parse_from([
        "census-service",
        "--store",
        root.to_str().ok_or("fixture path is not UTF-8")?,
        "--authorized-host",
        "127.0.0.1",
        "--delay-ms",
        "0",
        "provider",
        "tfrrs",
        "--offline",
        "--school-names",
        URL,
        "--observed-on",
        "2099-01-01",
        "--as-of",
        "2026-10-07",
    ])?)
}

fn isolated_fetcher(cli: &Cli, store: &Store, root: &Path) -> TestResult<Fetcher> {
    Ok(build_provider_fetcher(cli, store, provider(cli)?)?
        .with_origin_locks(root.join("origin-locks")))
}

fn online_cli(root: &Path) -> TestResult<Cli> {
    Ok(Cli::try_parse_from([
        "census-service",
        "--store",
        root.to_str().ok_or("fixture path is not UTF-8")?,
        "--authorized-host",
        "127.0.0.1",
        "--delay-ms",
        "0",
        "provider",
        "tfrrs",
    ])?)
}

fn fixture_origin_fence(root: &Path) -> TestResult<File> {
    let locks = root.join("origin-locks");
    std::fs::create_dir_all(&locks)?;
    let mut fence = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(locks.join(census_crawl::net::origin_lock_file_name(
            "https://indiana.tfrrs.org",
        )))?;
    fence.try_lock()?;
    fence.write_all(br#"{"fixture":"external-network safety fence"}"#)?;
    fence.flush()?;
    Ok(fence)
}

struct LoopbackObserver {
    listener: TcpListener,
    pending: std::net::TcpListener,
    origin: String,
}

impl LoopbackObserver {
    fn bind() -> TestResult<Self> {
        let pending = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        pending.set_nonblocking(true)?;
        let origin = format!("http://{}", pending.local_addr()?);
        let listener = TcpListener::from_std(pending.try_clone()?)?;
        Ok(Self {
            listener,
            pending,
            origin,
        })
    }

    fn require_idle(&self) -> TestResult {
        match self.pending.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(error.into()),
            Ok((_, peer)) => Err(format!("unexpected queued loopback dispatch from {peer}").into()),
        }
    }

    async fn without_dispatch<F, T>(&self, request: F) -> TestResult<T>
    where
        F: Future<Output = T>,
    {
        let outcome = tokio::select! {
            biased;
            accepted = self.listener.accept() => {
                let (_, peer) = accepted?;
                return Err(format!("unexpected loopback dispatch from {peer}").into());
            }
            outcome = tokio::time::timeout(REQUEST_DEADLINE, request) => outcome?,
        };
        self.require_idle()?;
        Ok(outcome)
    }

    async fn exchange<F, T>(&self, request: F, response: &str) -> TestResult<(T, ObservedRequest)>
    where
        F: Future<Output = T>,
    {
        let (outcome, observed) = tokio::time::timeout(REQUEST_DEADLINE, async {
            tokio::join!(request, self.answer_once(response))
        })
        .await?;
        let observed = observed?;
        self.require_idle()?;
        Ok((outcome, observed))
    }

    async fn answer_once(&self, body: &str) -> TestResult<ObservedRequest> {
        let (stream, _) = self.listener.accept().await?;
        let mut reader = BufReader::new(stream);
        let observed = ObservedRequest::read(&mut reader).await?;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nx-restate-id: offline-dispatch-fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        reader.get_mut().write_all(response.as_bytes()).await?;
        reader.get_mut().shutdown().await?;
        Ok(observed)
    }

    fn browser_lane(&self) -> TestResult<BrowserLane> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(REQUEST_DEADLINE)
            .build()?;
        Ok(BrowserLane::over(census_crawl::ingress::client(
            self.origin.parse()?,
            client,
        )?))
    }
}

struct ObservedRequest {
    line: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl ObservedRequest {
    async fn read(reader: &mut BufReader<TcpStream>) -> TestResult<Self> {
        let line = bounded_line(reader).await?;
        let mut headers: BTreeMap<String, String> = BTreeMap::new();
        for _ in 0..32 {
            let header = bounded_line(reader).await?;
            if header.is_empty() {
                let length = match headers.get("content-length") {
                    Some(length) => length.parse::<usize>()?,
                    None => 0,
                };
                if length > 4096 {
                    return Err("loopback request body exceeds fixture bound".into());
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).await?;
                return Ok(Self {
                    line,
                    headers,
                    body,
                });
            }
            let (name, value) = header.split_once(':').ok_or("malformed loopback header")?;
            if headers
                .insert(name.to_ascii_lowercase(), value.trim().to_string())
                .is_some()
            {
                return Err("duplicate loopback request header".into());
            }
        }
        Err("loopback request exceeds 32 headers".into())
    }
}

async fn bounded_line(reader: &mut BufReader<TcpStream>) -> TestResult<String> {
    let mut line = String::new();
    let bytes = reader.take(1024).read_line(&mut line).await?;
    if bytes == 0 || !line.ends_with("\r\n") {
        return Err("truncated or oversized loopback request line".into());
    }
    Ok(line.trim_end_matches("\r\n").to_string())
}

#[derive(Clone, Copy)]
enum HttpEntry {
    Get,
    Json,
    Form,
}

impl HttpEntry {
    async fn request(self, fetcher: &Fetcher, url: &str) -> Result<FetchOutcome, FetchError> {
        let options = FetchOptions::default();
        match self {
            Self::Get => fetcher.get(url, &options).await,
            Self::Json => {
                fetcher
                    .post_json(url, &serde_json::json!({"fixture": true}), &options)
                    .await
            }
            Self::Form => {
                fetcher
                    .post_form(url, &[("fixture".into(), "true".into())], &options)
                    .await
            }
        }
    }

    fn check(self, request: &ObservedRequest) -> TestResult {
        let (method, body, content_type) = match self {
            Self::Get => ("GET", "", None),
            Self::Json => ("POST", r#"{"fixture":true}"#, Some("application/json")),
            Self::Form => (
                "POST",
                "fixture=true",
                Some("application/x-www-form-urlencoded"),
            ),
        };
        check!(eq; request.line, format!("{method} /probe HTTP/1.1"));
        check!(eq; request.body.as_slice(), body.as_bytes());
        check!(eq; request.headers.get("content-type").map(String::as_str), content_type);
        Ok(())
    }
}

fn provider(cli: &Cli) -> TestResult<&ProviderArgs> {
    match &cli.command {
        Command::Provider(args) => Ok(args),
        other => Err(format!("expected provider command, got {other:?}").into()),
    }
}

fn cache_files(fetcher: &Fetcher) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    std::fs::read_dir(fetcher.cache_dir())?
        .map(|entry| {
            let path = entry?.path();
            Ok((path.clone(), std::fs::read(path)?))
        })
        .collect()
}

fn require_offline(outcome: Result<FetchOutcome, FetchError>, expected: &str) -> TestResult {
    match outcome {
        Err(FetchError::Offline { url }) => check!(eq; url, expected),
        other => return Err(format!("expected offline refusal, got {other:?}").into()),
    }
    Ok(())
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    args: &ProviderArgs,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher,
        store,
        refresh: args.refresh,
        school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: args.as_of.ok_or("fixture cutoff")?,
        recording: None,
    })
}

#[test]
fn provider_offline_requires_an_explicit_store() -> TestResult {
    let error = Cli::try_parse_from(["census-service", "provider", "tfrrs", "--offline"])
        .err()
        .ok_or("offline provider accepted the implicit default store")?;
    check!(eq; error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    check!(error.to_string().contains("--store"));
    Ok(())
}

#[test]
fn provider_offline_refuses_refresh_before_store_access() -> TestResult {
    let error = Cli::try_parse_from([
        "census-service",
        "--store",
        "unused-fixture-store",
        "provider",
        "tfrrs",
        "--offline",
        "--refresh",
    ])
    .err()
    .ok_or("offline provider accepted refresh")?;
    check!(eq; error.kind(), clap::error::ErrorKind::ArgumentConflict);
    check!(error.to_string().contains("--offline"));
    check!(error.to_string().contains("--refresh"));
    Ok(())
}

#[test]
fn provider_offline_accepts_the_explicit_store_after_the_subcommand() -> TestResult {
    let cli = Cli::try_parse_from([
        "census-service",
        "provider",
        "tfrrs",
        "--offline",
        "--store",
        "unused-fixture-store",
    ])?;
    check!(eq; provider(&cli)?.offline, true);
    check!(eq; cli.store, Some(PathBuf::from("unused-fixture-store")));
    Ok(())
}

#[test]
fn provider_store_alone_preserves_the_existing_network_capable_default() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = Cli::try_parse_from([
        "census-service",
        "provider",
        "tfrrs",
        "--store",
        directory
            .path()
            .to_str()
            .ok_or("fixture path is not UTF-8")?,
    ])?;
    let args = provider(&cli)?;
    check!(eq; args.offline, false);
    check!(eq; build_provider_fetcher(&cli, &store, args)?.is_offline(), false);
    Ok(())
}

#[tokio::test]
async fn provider_offline_refuses_all_http_entrypoints_without_dispatch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = offline_cli(directory.path())?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?;
    let observer = LoopbackObserver::bind()?;
    let url = format!("{}/probe", observer.origin);
    check!(eq; fetcher.is_offline(), true);
    for entry in [HttpEntry::Get, HttpEntry::Json, HttpEntry::Form] {
        let outcome = observer
            .without_dispatch(entry.request(&fetcher, &url))
            .await?;
        require_offline(outcome, &url)?;
    }
    let stats = fetcher.stats().await;
    check!(eq; stats.requests, 0);
    check!(eq; stats.cache_hits, 0);
    check!(eq; stats.physical_requests(), 0);
    check!(eq; stats.per_host.len(), 0);
    check!(eq; cache_files(&fetcher)?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn provider_without_offline_dispatches_every_http_entrypoint_to_the_observer() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = online_cli(directory.path())?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?;
    let observer = LoopbackObserver::bind()?;
    let url = format!("{}/probe", observer.origin);
    check!(eq; fetcher.is_offline(), false);
    for entry in [HttpEntry::Get, HttpEntry::Json, HttpEntry::Form] {
        let (outcome, request) = observer
            .exchange(entry.request(&fetcher, &url), "observed")
            .await?;
        let fetched = outcome?;
        check!(eq; fetched.body.as_slice(), b"observed");
        check!(eq; fetched.from_cache, false);
        entry.check(&request)?;
    }
    check!(eq; fetcher.stats().await.physical_requests(), 3);
    Ok(())
}

#[tokio::test]
async fn provider_offline_refuses_browser_cache_miss_without_ingress_dispatch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = offline_cli(directory.path())?;
    let observer = LoopbackObserver::bind()?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?
        .with_browser_lane(observer.browser_lane()?);
    check!(eq; fetcher.is_offline(), true);
    check!(eq; fetcher.has_browser_lane(), true);
    let options = FetchOptions::default();
    let outcome = observer
        .without_dispatch(fetcher.get(BROWSER_URL, &options))
        .await?;
    require_offline(outcome, BROWSER_URL)?;
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(eq; cache_files(&fetcher)?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn provider_without_offline_dispatches_to_the_browser_ingress_observer() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = online_cli(directory.path())?;
    let observer = LoopbackObserver::bind()?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?
        .with_browser_lane(observer.browser_lane()?);
    check!(eq; fetcher.is_offline(), false);
    let options = FetchOptions::default();
    let (outcome, request) = observer
        .exchange(fetcher.get(BROWSER_URL, &options), BROWSER_CAPTURE)
        .await?;
    let fetched = outcome?;
    check!(eq; fetched.body.as_slice(), b"observed");
    check!(eq; fetched.from_cache, false);
    check!(eq; request.line, "POST /BrowserSession/profile-0/fetch HTTP/1.1");
    let spec: RequestSpec = serde_json::from_slice(&request.body)?;
    check!(eq; spec.url, BROWSER_URL);
    check!(eq; spec.semantic_url, BROWSER_URL);
    check!(eq; spec.action, Action::Fetch {});
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn provider_offline_reuses_cached_bytes_and_original_acquisition_metadata() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = offline_cli(directory.path())?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?;
    let observer = LoopbackObserver::bind()?;
    let url = format!("{}/cached", observer.origin);
    check!(eq; fetcher.is_offline(), true);
    capture_cache::seed(fetcher.cache_dir(), &url, LIST, ACQUIRED_AT, &[])?;
    let retained = cache_files(&fetcher)?;
    let options = FetchOptions::default();
    let fetched = observer
        .without_dispatch(fetcher.get(&url, &options))
        .await??;
    check!(eq; fetched.body.as_slice(), LIST);
    check!(eq; fetched.url, url);
    check!(eq; fetched.fetched_at, ACQUIRED_AT);
    check!(eq; fetched.from_cache, true);
    check!(eq; cache_files(&fetcher)?, retained);
    let stats = fetcher.stats().await;
    check!(eq; stats.requests, 1);
    check!(eq; stats.cache_hits, 1);
    check!(eq; stats.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn provider_offline_refuses_corrupt_cached_bytes_without_replacing_them() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = offline_cli(directory.path())?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?;
    let observer = LoopbackObserver::bind()?;
    let url = format!("{}/corrupt", observer.origin);
    check!(eq; fetcher.is_offline(), true);
    capture_cache::seed(fetcher.cache_dir(), &url, LIST, ACQUIRED_AT, &[])?;
    let body = cache_files(&fetcher)?
        .into_keys()
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "body")
        })
        .ok_or("cached fixture body missing")?;
    std::fs::write(body, b"corrupt fixture")?;
    let retained = cache_files(&fetcher)?;
    let options = FetchOptions::default();
    let outcome = observer
        .without_dispatch(fetcher.get(&url, &options))
        .await?;
    require_offline(outcome, &url)?;
    check!(eq; cache_files(&fetcher)?, retained);
    let stats = fetcher.stats().await;
    check!(eq; stats.requests, 0);
    check!(eq; stats.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn provider_offline_uses_the_current_collector_and_replays_physical_rows_once() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = offline_cli(directory.path())?;
    let args = provider(&cli)?;
    let _fence = fixture_origin_fence(directory.path())?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?;
    check!(eq; fetcher.is_offline(), true);
    capture_cache::seed(fetcher.cache_dir(), URL, LIST, ACQUIRED_AT, &[])?;
    let retained = cache_files(&fetcher)?;
    let initial_context = context(&store, &fetcher, args)?;
    let report = dispatch(&initial_context, args, initial_context.observed_on.clone()).await?;
    check!(eq; report.errors, 0);
    check!(eq; report.disposition, CollectionDisposition::Complete);
    check!(eq; report.requests, 0);
    check!(eq; report.from_cache, 1);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let control = performances
        .iter()
        .find(|row| {
            row.source_athlete
                .as_ref()
                .is_some_and(|owner| owner.id == "8817192")
        })
        .ok_or("published control performance")?;
    check!(eq; control.mark, Mark::TimeSeconds(ExactSeconds::parse("6.97")?));
    check!(control
        .evidence
        .iter()
        .any(|evidence| evidence.observed_on == ACQUIRED_AT));
    let physical = store.walk_table(Table::Performances)?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    let replay_fetcher = isolated_fetcher(&cli, &reopened, directory.path())?;
    check!(eq; replay_fetcher.is_offline(), true);
    let replay_context = context(&reopened, &replay_fetcher, args)?;
    let replay = dispatch(&replay_context, args, replay_context.observed_on.clone()).await?;
    check!(eq; replay.errors, 0);
    check!(eq; replay.disposition, CollectionDisposition::Complete);
    check!(eq; reopened.walk_table(Table::Performances)?, physical);
    check!(eq; cache_files(&fetcher)?, retained);
    Ok(())
}

#[tokio::test]
async fn provider_offline_missing_capture_keeps_the_collector_unfinished() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let cli = offline_cli(directory.path())?;
    let args = provider(&cli)?;
    let _fence = fixture_origin_fence(directory.path())?;
    let fetcher = isolated_fetcher(&cli, &store, directory.path())?;
    check!(eq; fetcher.is_offline(), true);
    let context = context(&store, &fetcher, args)?;
    let report = dispatch(&context, args, context.observed_on.clone()).await?;
    check!(eq; report.rows, 0);
    check!(eq; report.errors, 1);
    check!(eq; report.unfinished, vec![URL.to_string()]);
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains(&format!("offline and not cached: {URL}"))));
    check!(eq; report.requests, 0);
    check!(eq; report.from_cache, 0);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    Ok(())
}
