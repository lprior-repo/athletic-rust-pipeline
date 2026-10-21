use super::*;
use crate::acquisition::facts::{Armed, Observed, ReceiptWitness, Reservation};
use crate::StoreError;
use census_domain::UsJurisdiction;
use serde_json::json;

mod backing;
mod fixture;
mod metadata;
mod prefix;
mod request;
mod response;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn transition_refused(result: crate::StoreResult<()>) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    Ok(())
}

fn backing_refused(result: crate::StoreResult<()>) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::Capture(CaptureError::BackingIntegrity))
    ));
    Ok(())
}
