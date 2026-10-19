use super::*;
use crate::export::ExportDataset;

pub(in super::super) fn reconcile(
    store: &Store,
    root: &Path,
    intent: &Intent,
    run_manifest: &Option<Digest>,
    dataset: Option<&ExportDataset>,
) -> ReportResult<RecoveryOutcome> {
    let current = match store.publication_lease(&intent.request.identity) {
        Err(census_store::StoreError::LeaseExpired { .. }) => {
            store.publication_rebind_lease(&intent.request)?
        }
        Err(error) => return Err(error.into()),
        Ok(_) => store
            .publication_read(&intent.request.identity)?
            .ok_or_else(|| super::super::invariant("publication request disappeared".into()))?,
    };
    if matches!(current.state, State::Requested) {
        return Ok(RecoveryOutcome::NoIntent);
    }
    let selected = pointer::read(root)?;
    let desired = current.desired().ok_or_else(|| {
        super::super::invariant("advanced publication request has no desired bundle".into())
    })?;
    super::dispatch_intent_recovery(
        store,
        root,
        &current,
        &selected,
        desired,
        run_manifest,
        dataset,
    )
}
