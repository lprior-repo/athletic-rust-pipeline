use super::{CapturedPublicDirectory, PublicResourceAcquisition};
use crate::net::{CaptureTransportError, FetchError, FetchOptions, Fetcher};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{PublicResourceCaptureOwnerV1, PublicResourceRootKindV1, Store};
use fixture::{Fixture, TestResult};
use tokio::net::TcpListener;

mod assertions;
mod binding;
mod dispatch_freshness;
mod fixture;
mod redirects;
mod refusals;
mod replay;
mod robots_dispatch;
mod roster;
mod server;
mod transport;

const ORIGIN: &str = "https://mi.milesplit.com";
const DIRECTORY: &[u8] = b"<!doctype html><title>Controlled Michigan directory</title>";
