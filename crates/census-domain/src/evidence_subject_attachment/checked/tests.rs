use super::*;
use crate::evidence_subject_attachment::tests::fixture::{
    command, digest, inventory, receipts, seal, stamp, store, text,
};

mod boundaries;
mod collections;
mod grants;
mod projection;
mod roundtrip;
mod support;

use support::*;
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
