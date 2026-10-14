use census_store::CaptureWriterV1;
use std::sync::Arc;

#[derive(Clone)]
pub enum CaptureExecution {
    LegacyFresh,
    FreshQualificationV1(Arc<CaptureWriterV1>),
}
