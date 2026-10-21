use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::discovery::{
    self as source_rows, OriginalResultSpan,
};
use crate::participation_evidence::result_ownership::{
    read_retained_html_result_rows, RetainedResultOwnershipError,
};
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use chrono::{DateTime, Utc};

mod genuine;
mod parser;
mod span;
mod support;
pub(in crate::participation_evidence) mod tfrrs;

pub(in crate::participation_evidence::recorded) use genuine::verify_genuine;
use support::*;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
