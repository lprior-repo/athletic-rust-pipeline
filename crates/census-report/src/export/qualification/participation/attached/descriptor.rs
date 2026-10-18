use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_store::StoreSnapshot;
use std::path::Path;

pub(super) fn capture(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    directory: &Path,
) -> ReportResult<super::super::archive::attachment::AttachmentClosureDescriptor> {
    let mut grant = Grant::new();
    let result = (|| {
        let accepted = dataset.verified_public_identities()?.accepted_snapshot();
        let current = snapshot
            .read_current_evidence_subject_attachment_references_with_accepted_v1(
                accepted, &mut grant,
            )?;
        let bundle = current.freeze_closure(&mut grant)?;
        super::super::archive::capture_attachment(&bundle, directory, &mut grant)
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}
