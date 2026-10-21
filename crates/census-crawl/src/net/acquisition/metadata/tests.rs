use super::*;
use census_domain::model::{
    AcquisitionAttemptId, AcquisitionDigest, AcquisitionDispatchId, AcquisitionWorkId,
    AttemptOrdinal, HopOrdinal, ObservedHttpHopV1, StoreIdentity,
};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::ResponseBuilderExt;
use serde_json::{json, Value};

mod binding;
mod fixture;
mod retained;
mod wire;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
const FINAL: &str = "https://mi.milesplit.com/athletes/13749840";
const START: &str = "2026-10-21T08:00:00Z";
const HEAD: &str = "2026-10-21T08:00:03+00:00";
const DONE: &str = "2026-10-21T08:00:04Z";

fn refused(bytes: &[u8], capture: &FetchOutcome) -> TestResult {
    check!(matches!(
        observed(bytes, capture),
        Err(FetchError::Capture(
            CaptureTransportError::BackingUnavailable
        ))
    ));
    Ok(())
}

fn malformed(bytes: &[u8], capture: &FetchOutcome) -> TestResult {
    check!(matches!(
        observed(bytes, capture),
        Err(FetchError::Decode { .. })
    ));
    Ok(())
}
