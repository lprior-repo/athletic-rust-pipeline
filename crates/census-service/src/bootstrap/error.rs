
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::outcome::DrainState;
use census_store::StoreError;

use super::USAGE;

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("{flag} needs a value\n{}", USAGE)]
    MissingValue { flag: String },
    #[error("{raw} is not a socket address")]
    ListenNotAnAddress {
        raw: String,
        #[source]
        source: std::net::AddrParseError,
    },
    #[error("{raw} is not a positive integer")]
    ConcurrencyNotANumber {
        raw: String,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("--max-concurrent must be at least 1")]
    ConcurrencyIsZero,
    #[error("{raw} is not a number of seconds")]
    DrainTimeoutNotASeconds {
        raw: String,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("{}", USAGE)]
    HelpRequested,
    #[error("unknown flag {flag}\n{}", USAGE)]
    UnknownFlag { flag: String },
    #[error(
        "--listen {listen} is not a loopback address: the endpoint has no identity key configured, \
         so it must not be reachable from another host"
    )]
    NonLoopbackListen { listen: SocketAddr },
    #[error("binding {listen} failed: {source}")]
    Bind {
        listen: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("reading the bound address failed: {source}")]
    BoundAddress {
        #[source]
        source: std::io::Error,
    },
    #[error("creating {path} failed: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("opening the store under {path} failed: {source}")]
    StoreOpen {
        path: PathBuf,
        #[source]
        source: StoreError,
    },
    #[error("joining the store bootstrap task failed: {reason}")]
    StoreTask { state: DrainState, reason: String },
    #[error("persisting the store during shutdown failed: {source}")]
    StoreFlush {
        #[source]
        source: StoreError,
    },
    #[error("subscribing to {signal} failed: {source}")]
    Signal {
        signal: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("task count does not fit u64")]
    TaskCountOverflow,
    #[error("the browser lane's origin {origin} is not a URL: {source}")]
    LaneOriginUnusable {
        origin: String,
        source: url::ParseError,
    },
    #[error("{flag} configures the browser lane, which --browser-profile enables")]
    LaneFlagWithoutProfile { flag: String },
    #[error("{program} is on no PATH entry: name the browser with --browser-executable")]
    LaneBrowserNotOnPath { program: String },
    #[error("building the browser lane's client for {origin} failed: {detail}")]
    LaneIngressUnusable { origin: String, detail: String },
}
