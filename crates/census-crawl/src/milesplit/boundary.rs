use std::future::Future;
use std::pin::Pin;

use crate::CrawlError;

pub const CHUNK_ROWS: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    BeforeSourceRequest,
    ResponseReceived,
    CaptureCommitted,
    PageChunk { index: u32 },
    BeforeApply,
    AfterCommitBeforeAck,
}

impl Point {
    pub fn kind(&self) -> &'static str {
        match self {
            Point::BeforeSourceRequest => "before_source_request",
            Point::ResponseReceived => "response_received",
            Point::CaptureCommitted => "capture_committed",
            Point::PageChunk { .. } => "page_chunk",
            Point::BeforeApply => "before_apply",
            Point::AfterCommitBeforeAck => "after_commit_before_ack",
        }
    }
}

pub trait Hook: Send + Sync {
    fn reached<'a>(
        &'a self,
        point: Point,
    ) -> Pin<Box<dyn Future<Output = Result<(), CrawlError>> + Send + 'a>>;
}
