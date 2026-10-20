use super::super::*;
use super::*;
use crate::export::ExportJobRequest;

#[test]
fn publication_recovery_requested_advanced_state_dispatches_actual_bundle() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::super::fence_tests::options(None)?;
    let dataset = ExportDataset::for_job_request(
        &store,
        "requested-advanced-state",
        &ExportJobRequest::LegacyV2,
    )?;
    let root = store.out_dir().join("publication");
    let _stage = Stage::begin(&dataset, &store, &options)?;
    let namespace = binding::namespace(&root)?;
    let requested = store
        .publication_requested(&namespace)?
        .ok_or("requested publication missing")?;
    let censuses =
        crate::workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    let mut desired_options = options.clone();
    desired_options.out = Some(temp.path().join("desired-build"));
    let workbook = crate::workbook::build_from(&dataset, &store, &desired_options, &censuses)?;
    let desired_dir = workbook.parent().ok_or("bundle directory")?;
    let desired = Digest::parse(
        desired_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("digest")?,
    )?;
    std::fs::rename(desired_dir, root.join("generations").join(desired.as_str()))?;
    sync_directory(&root.join("generations"))?;
    let desired_workbook = root
        .join("generations")
        .join(desired.as_str())
        .join("workbook.xlsx");
    let desired_bytes = std::fs::read(&desired_workbook)?;
    let run_manifest = binding::run_manifest(&store)?;
    let advanced = {
        let fence = store.publication_fence(&requested.request, &run_manifest)?;
        fence.begin(fence.intent(), &desired)?
    };
    check!(eq; advanced.state, State::Intent { desired: desired.clone() });
    store.lease_renew(
        requested.request.identity.as_str(),
        advanced.request.lease_epoch,
        &super::expiring_at()?,
    )?;
    super::await_lease_expiry(&store, &requested.request.identity)?;
    let outcome = {
        let _namespace = lock_publication(&root)?;
        super::super::recovery::requested::reconcile(
            &store,
            &root,
            &requested,
            &run_manifest,
            Some(&dataset),
        )?
    };
    check!(eq; outcome, RecoveryOutcome::ExpectedRetried { bundle: desired.clone() });
    check!(eq; pointer::read(&root)?, Root::Bundle(desired.clone()));
    let retained = store
        .publication_read(&requested.request.identity)?
        .ok_or("advanced publication missing")?;
    check!(eq; retained.state, State::Published { receipt: desired.clone() });
    check!(eq; std::fs::read(&desired_workbook)?, desired_bytes);
    check!(
        eq;
        recover_publication(&store, &root)?,
        RecoveryOutcome::HistoricalSuccess { bundle: desired.clone() }
    );
    check!(eq; pointer::read(&root)?, Root::Bundle(desired.clone()));
    check!(eq; std::fs::read(&desired_workbook)?, desired_bytes);
    Ok(())
}
