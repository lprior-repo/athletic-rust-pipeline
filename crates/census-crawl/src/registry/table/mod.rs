use super::SourceDescriptor;

mod from_mshsl;
mod through_milesplit;

use from_mshsl::FROM_MSHSL;
mod directories;
use directories::DIRECTORIES;
use through_milesplit::THROUGH_MILESPLIT;

pub fn descriptors() -> impl Iterator<Item = &'static SourceDescriptor> {
    THROUGH_MILESPLIT
        .iter()
        .chain(FROM_MSHSL.iter())
        .chain(DIRECTORIES.iter())
}
