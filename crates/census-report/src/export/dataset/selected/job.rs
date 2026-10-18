use super::*;

impl ExportDataset {
    pub(crate) fn ensure_job_commitment(&self) -> ReportResult<()> {
        if self.lineage.export_job.is_some() && self.selected_job_freshness.is_none() {
            return Err(invalid(
                "selected job publication requires its verified durable job commitment",
            ));
        }
        Ok(())
    }
}
