use super::SourceDescriptor;

mod directories;
mod from_mshsl;
mod through_milesplit;

use directories::DIRECTORIES;
use from_mshsl::FROM_MSHSL;
use through_milesplit::THROUGH_MILESPLIT;

pub fn descriptors() -> impl Iterator<Item = &'static SourceDescriptor> {
    THROUGH_MILESPLIT
        .iter()
        .chain(FROM_MSHSL.iter())
        .chain(DIRECTORIES.iter())
}
