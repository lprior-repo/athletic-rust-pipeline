use super::super::{invalid, SelectedProofs};
use crate::report::ReportResult;
use std::path::Path;

impl SelectedProofs {
    pub(crate) fn object_bytes(&self) -> ReportResult<u64> {
        self.validate()?;
        self.objects.iter().try_fold(0_u64, |total, object| {
            total
                .checked_add(object.bytes)
                .ok_or_else(|| invalid("proof inventory byte count overflow"))
        })
    }

    pub(crate) fn verify_objects(&self, directory: &Path) -> ReportResult<()> {
        self.validate()?;
        super::verify(directory, &self.objects)
    }

    pub(crate) fn bind_directory(&mut self, directory: &Path) -> ReportResult<()> {
        self.validate()?;
        super::verify(directory, &self.objects)?;
        self.objects_directory = Some(directory.to_path_buf());
        Ok(())
    }

    pub(crate) fn copy_objects(&self, destination: &Path) -> ReportResult<()> {
        self.validate()?;
        let source = self
            .objects_directory
            .as_deref()
            .ok_or_else(|| invalid("selected proof objects have no retained directory"))?;
        super::copy(source, destination, &self.objects)
    }
}
