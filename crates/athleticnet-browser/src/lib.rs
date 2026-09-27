#![forbid(unsafe_code)]

use std::time::Duration;

mod actor;
pub mod challenge;
pub mod clock;
pub mod drain;
mod lifecycle;
pub(crate) mod navigation;
mod ops;
mod outcome;
mod pool;
mod profile;
pub mod protocol;
pub mod request;
mod response_wire;
pub mod retry;
pub(crate) mod transport;
pub use lifecycle::error::BrowserStartupError;
pub use lifecycle::BrowserManager;
pub use outcome::{
    BrowserCapture, BrowserError, BrowserFailure, BrowserOutcome, BrowserResponse, Verdict,
};
pub use profile::{BrowserSettings, BrowserState, BrowserStatus};
pub(crate) mod gate;

pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);
