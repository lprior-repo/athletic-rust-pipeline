mod leaf;
mod read;
mod root;

pub use root::{RetainedCaptureRootSchemaV1, RetainedCaptureRootV1};

pub use leaf::{RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureSlotV1};

pub use read::RetainedCaptureRootReadV1;

pub(crate) const MAX_RETAINED_CAPTURE_ENVELOPE_BYTES: usize = 2 * 1024 * 1024;
