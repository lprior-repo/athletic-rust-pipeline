mod leaf;
mod read;
mod root;

pub use root::{RetainedCaptureRootSchemaV1, RetainedCaptureRootV1};

pub use leaf::{RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureSlotV1};

pub use read::RetainedCaptureRootReadV1;

pub(crate) const MAX_RETAINED_CAPTURE_ENVELOPE_BYTES: usize = 4 * 1024 * 1024;

pub(crate) use leaf::verify as verify_leaf;
pub(crate) use read::owned as read_owned;
pub(crate) use read::{with_checked, CheckedCapture, CheckedRoot, CheckedSchool};
