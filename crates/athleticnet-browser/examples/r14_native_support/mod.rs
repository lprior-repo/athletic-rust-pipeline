use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use url::Url;

use crate::fixture_input;
mod wire;
use wire::serve;

type Support<T> = Result<T, SupportError>;

const HEAD_LIMIT: usize = 8192;
const READ_UNIT: usize = 1024;
const WRITE_UNIT: usize = 65_536;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const READ_TIMEOUT: Duration = Duration::from_secs(2);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const OK_BODY: &[u8] = b"wire-smoke-ok";
const BOOTSTRAP: &[u8] = b"<!doctype html><title>r14 native support</title>";
const HTML_TYPE: &str = "text/html; charset=utf-8";
const TEXT_TYPE: &str = "text/plain";
const NO_CONTENT_HEAD: &str = "HTTP/1.1 204 No Content\r\nconnection: close\r\n\r\n";
const TOO_LARGE_REASON: &str = "Request Header Fields Too Large";

#[derive(Debug)]
enum SupportError {
    Io {
        context: &'static str,
        source: io::Error,
    },
    Origin(url::ParseError),
    Detail(String),
    FixtureInput(fixture_input::FixtureInputError),
}
impl fmt::Display for SupportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(formatter, "r14 {context}: {source}"),
            Self::Origin(source) => write!(formatter, "origin: {source}"),
            Self::Detail(detail) => write!(formatter, "r14 support: {detail}"),
            Self::FixtureInput(source) => write!(formatter, "r14 fixture input: {source}"),
        }
    }
}
impl Error for SupportError {}

struct Fixtures {
    wire_over: Vec<u8>,
    decoded_over: Vec<u8>,
}

fn read_fixture(directory: &Path) -> Support<Fixtures> {
    let manifest = fixture_input::read_manifest(&directory.join("manifest.json"))
        .map_err(SupportError::FixtureInput)?;
    let wire_over = fixture_input::read_artifact(
        &directory.join("wire-over.gz"),
        &manifest.wire,
        fixture_input::ArtifactKind::Wire,
    )
    .map_err(SupportError::FixtureInput)?;
    let decoded_over = fixture_input::read_artifact(
        &directory.join("decoded-over.gz"),
        &manifest.decoded,
        fixture_input::ArtifactKind::Decoded,
    )
    .map_err(SupportError::FixtureInput)?;
    Ok(Fixtures {
        wire_over,
        decoded_over,
    })
}

fn fault(context: &'static str) -> impl FnOnce(io::Error) -> SupportError {
    move |source| SupportError::Io { context, source }
}

pub struct Server {
    origin: Url,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Support<BTreeMap<String, u64>>>>,
}
impl Server {
    pub fn start(fixtures: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let fixtures = read_fixture(fixtures)?;
        let listener = TcpListener::bind("127.0.0.1:0").map_err(fault("bind"))?;
        let address = listener.local_addr().map_err(fault("local address"))?;
        listener
            .set_nonblocking(true)
            .map_err(fault("set nonblocking"))?;
        let origin = Url::parse(&format!("http://{address}/")).map_err(SupportError::Origin)?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("r14-native-support".to_owned())
            .spawn(move || serve(listener, fixtures, worker_stop))
            .map_err(fault("spawn"))?;
        Ok(Self {
            origin,
            stop,
            worker: Some(worker),
        })
    }
    pub fn origin(&self) -> &Url {
        &self.origin
    }
    fn stop_worker(&mut self) -> Option<JoinHandle<Support<BTreeMap<String, u64>>>> {
        let worker = self.worker.take()?;
        self.stop.store(true, Ordering::SeqCst);
        worker.thread().unpark();
        Some(worker)
    }
    pub fn finish(mut self) -> Result<BTreeMap<String, u64>, Box<dyn Error + Send + Sync>> {
        let Some(worker) = self.stop_worker() else {
            return Err(Box::new(SupportError::Detail(
                "already finished".to_owned(),
            )));
        };
        match worker.join() {
            Ok(counters) => counters.map_err(Box::<dyn Error + Send + Sync>::from),
            Err(_) => Err(Box::new(SupportError::Detail("thread panicked".to_owned()))),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(worker) = self.stop_worker() {
            match worker.join() {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => eprintln!("r14 native support cleanup failed: {error}"),
                Err(_panic) => eprintln!("r14 native support thread panicked"),
            }
        }
    }
}
