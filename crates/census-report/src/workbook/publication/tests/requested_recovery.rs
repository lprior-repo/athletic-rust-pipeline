use super::*;
use census_store::publish::{Digest, Intent, Request, Root, State};

fn expiring_requested(
    store: &Store,
    root: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> TestResult<Intent> {
    let namespace = binding::namespace(root)?;
    let input = binding::input(dataset, store, options)?;
    let identity =
        binding::digest(&(&namespace, &dataset.lineage.input_generation, &input.recipe))?;
    let expires = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("lease expiry overflow")?
        .to_rfc3339();
    let lease_epoch = store.lease_acquire(identity.as_str(), "workbook-publication", &expires)?;
    let intent = store.publication_request(Request {
        namespace,
        identity,
        expected: pointer::read(root)?,
        input,
        lease_epoch,
    })?;
    await_expired(store, &intent.request.identity)?;
    Ok(intent)
}

fn await_expired(store: &Store, identity: &Digest) -> TestResult {
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(30))
        .ok_or("lease wait deadline overflow")?;
    loop {
        match store.publication_lease(identity) {
            Err(census_store::StoreError::LeaseExpired { .. }) => return Ok(()),
            Err(error) => return Err(error.into()),
            Ok(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Ok(_) => return Err("publication lease did not expire".into()),
        }
    }
}

fn select_other_bundle(
    store: &Store,
    root: &Path,
    other_root: PathBuf,
    options: &Options,
) -> TestResult<PathBuf> {
    let mut other = options.clone();
    other.limit = Some(2);
    other.out = Some(other_root);
    let workbook = crate::workbook::build(store, &other)?;
    let generation = workbook.parent().ok_or("other generation missing")?;
    let name = generation.file_name().ok_or("other digest missing")?;
    let digest = Digest::parse(name.to_str().ok_or("other digest is not UTF-8")?)?;
    let installed = root.join("generations").join(name);
    std::fs::rename(generation, &installed)?;
    pointer::switch(root, &digest)?;
    sync_directory(root)?;
    Ok(installed.join("workbook.xlsx"))
}

#[test]
fn expired_requested_stage_cannot_replace_unconsumed_integrity_history() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let mut options = super::super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let root = store.out_dir().join("publication");
    let namespace = binding::namespace(&root)?;
    let active = store
        .publication_active(&namespace)?
        .ok_or("active publication missing")?;
    let active_bundle = active.desired().ok_or("active bundle missing")?.clone();
    let selected = select_other_bundle(&store, &root, temp.path().join("other"), &options)?;
    let selected_bytes = std::fs::read(&selected)?;
    options.limit = Some(1);
    let dataset = ExportDataset::for_job(&store, "requested-integrity-history")?;
    let request = expiring_requested(&store, &root, &dataset, &options)?;
    let lease = store.get_lease_record(request.request.identity.as_str())?;
    let corrupt = old
        .parent()
        .ok_or("old generation missing")?
        .join("recruiting.csv");
    std::fs::write(&corrupt, "retained corrupt artifact")?;
    let error = match Stage::begin(&dataset, &store, &options) {
        Err(error) => error,
        Ok(_) => return Err("Requested rebind discarded an unconsumed integrity incident".into()),
    };
    check!(matches!(
        error,
        ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. })
            if bundle == active_bundle
    ));
    check!(eq; store.publication_active(&namespace)?, Some(active.clone()));
    check!(eq; store.publication_read(&request.request.identity)?, Some(request.clone()));
    check!(eq; store.get_lease_record(request.request.identity.as_str())?, lease);
    check!(eq; current_workbook(&root)?, selected);
    check!(eq; std::fs::read(&selected)?, selected_bytes);
    check!(eq; std::fs::read(&corrupt)?, b"retained corrupt artifact");
    check!(matches!(
        incident::check_root(&root),
        Err(ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. }))
            if bundle == active_bundle
    ));
    assert_no_temporaries(&root)?;
    store.flush()?;
    drop(store);
    let reopened = Store::open(temp.path().join("store"))?;
    check!(eq; reopened.publication_active(&namespace)?, Some(active));
    check!(eq; reopened.publication_read(&request.request.identity)?, Some(request.clone()));
    check!(eq; reopened.get_lease_record(request.request.identity.as_str())?, lease);
    check!(eq; current_workbook(&root)?, selected);
    check!(eq; std::fs::read(&selected)?, selected_bytes);
    Ok(())
}

#[test]
fn requested_stage_without_active_history_rebinds_exactly_once_before_publication() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::super::fence_tests::options(None)?;
    let dataset = ExportDataset::for_job(&store, "requested-without-active")?;
    let root = store.out_dir().join("publication");
    let request = expiring_requested(&store, &root, &dataset, &options)?;
    check!(eq; store.publication_active(&request.request.namespace)?, None);
    let stage = Stage::begin(&dataset, &store, &options)?;
    let rebound = store
        .publication_read(&request.request.identity)?
        .ok_or("rebound request missing")?;
    let next_epoch = request
        .request
        .lease_epoch
        .checked_add(1)
        .ok_or("epoch overflow")?;
    check!(eq; rebound.request.lease_epoch, next_epoch);
    check!(eq; rebound.state, State::Requested);
    let mut expected = request.request.clone();
    expected.lease_epoch = next_epoch;
    check!(eq; rebound.request, expected);
    check!(eq; store.publication_active(&request.request.namespace)?, Some(rebound));
    check!(eq; pointer::read(&root)?, Root::Absent);
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let published = stage.publish(&dataset, &options, |directory| {
        crate::workbook::write_artifacts(directory, &dataset, &options, &censuses)
    })?;
    let retained = store
        .publication_read(&request.request.identity)?
        .ok_or("published request missing")?;
    check!(eq; retained.request.lease_epoch, next_epoch);
    check!(matches!(retained.state, State::Published { .. }));
    check!(eq; store.publication_active(&request.request.namespace)?, Some(retained));
    check!(eq; current_workbook(&root)?, published);
    verify_published(&published)?;
    assert_no_temporaries(&root)?;
    Ok(())
}
