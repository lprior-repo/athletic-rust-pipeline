use census_store::CaptureWriterV1;
use std::sync::Arc;

#[derive(Clone)]
pub enum CaptureExecution {
    LegacyFresh,
    FreshQualification(Arc<CaptureWriterV1>),
}
