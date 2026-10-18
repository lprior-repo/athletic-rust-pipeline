use super::*;
use crate::export::ExportDataset;

pub(in crate::workbook::publication) fn reconcile(
    store: &Store,
    root: &Path,
    namespace: &Digest,
    prepared: &Prepared,
) -> ReportResult<RecoveryOutcome> {
    super::reconcile_with_dataset(store, root, namespace, prepared, None)
}

pub(super) fn selected_input<'a>(
    root: &Path,
    desired: &Digest,
    dataset: Option<&'a ExportDataset>,
) -> ReportResult<Option<&'a ExportDataset>> {
    let manifest =
        super::super::manifest::verify_directory(&root.join("generations").join(desired.as_str()))?;
    if !manifest.is_selected_job() {
        return Ok(None);
    }
    let dataset = dataset
        .ok_or_else(|| refused("selected job recovery requires its verified durable job input"))?;
    if dataset.lineage != manifest.lineage {
        return Err(refused(
            "selected job recovery input differs from its retained generation",
        ));
    }
    dataset.ensure_current_selected_write()?;
    Ok(Some(dataset))
}

pub(super) fn resume_selected(
    store: &Store,
    root: &Path,
    intent: &Intent,
    selected: &Root,
    desired: &Digest,
    run_manifest: &Option<Digest>,
    dataset: &ExportDataset,
) -> ReportResult<RecoveryOutcome> {
    if *selected != intent.request.expected && *selected != Root::Bundle(desired.clone()) {
        return Err(refused("selected job recovery pointer changed"));
    }
    let fence = store.publication_fence(&intent.request, run_manifest)?;
    dataset
        .ensure_snapshot(&fence.snapshot())
        .map_err(super::super::binding::input_refused)?;
    if *selected == Root::Bundle(desired.clone()) {
        super::super::sync_directory(root).map_err(|error| protocol::uncertain(intent, error))?;
        fence
            .published(fence.intent(), desired)
            .map_err(|error| protocol::uncertain(intent, error.into()))?;
        return Ok(RecoveryOutcome::DesiredPublished {
            bundle: desired.clone(),
        });
    }
    pointer::switch(root, desired)?;
    pointer::finish(root, fence.intent(), || {
        fence
            .published(fence.intent(), desired)
            .map(|_| ())
            .map_err(Into::into)
    })?;
    Ok(RecoveryOutcome::ExpectedRetried {
        bundle: desired.clone(),
    })
}

fn refused(detail: &str) -> crate::report::ReportError {
    PublicationError::InputFenceRefused {
        detail: detail.into(),
    }
    .into()
}

pub(super) fn validate_pre_reconcile(
    store: &Store,
    root: &Path,
    namespace: &Digest,
    prepared: &Prepared,
) -> ReportResult<()> {
    if store.publication_active(namespace)? != prepared.active
        || pointer::read(root).map_err(|error| error.to_string()) != prepared.observed
    {
        return Err(pointer::required(
            root,
            "publication changed during pre-fence validation; retry explicit recovery".into(),
            Vec::new(),
        ));
    }
    Ok(())
}
