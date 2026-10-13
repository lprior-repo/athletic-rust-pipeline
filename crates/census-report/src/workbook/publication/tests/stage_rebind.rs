use super::*;
use census_store::publish::{Digest, Intent, Request, State};

struct Retained {
    temp: tempfile::TempDir,
    store: Store,
    dataset: ExportDataset,
    options: Options,
    request: Intent,
    old: PathBuf,
    old_bytes: Vec<u8>,
    root: PathBuf,
}

impl Retained {
    fn new() -> TestResult<Self> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path().join("store"))?;
        let mut options = super::super::fence_tests::options(None)?;
        let old = crate::workbook::build(&store, &options)?;
        let old_bytes = std::fs::read(&old)?;
        options.limit = Some(1);
        let dataset = ExportDataset::for_job(&store, "requested-stage-rebind")?;
        let root = store.out_dir().join("publication");
        let namespace = binding::namespace(&root)?;
        let input = binding::input(&dataset, &store, &options)?;
        let identity =
            binding::digest(&(&namespace, &dataset.lineage.input_generation, &input.recipe))?;
        let expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::seconds(1))
            .ok_or("lease expiry overflow")?
            .to_rfc3339();
        let lease_epoch =
            store.lease_acquire(identity.as_str(), "workbook-publication", &expires)?;
        let request = store.publication_request(Request {
            namespace,
            identity,
            expected: pointer::read(&root)?,
            input,
            lease_epoch,
        })?;
        await_expired(&store, &request.request.identity)?;
        Ok(Self {
            temp,
            store,
            dataset,
            options,
            request,
            old,
            old_bytes,
            root,
        })
    }

    fn unchanged_previous(&self) -> TestResult {
        check!(eq; current_workbook(&self.root)?, self.old);
        check!(eq; std::fs::read(&self.old)?, self.old_bytes);
        verify_published(&self.old)?;
        Ok(())
    }

    fn rebound(&self) -> TestResult<Intent> {
        let current = self
            .store
            .publication_read(&self.request.request.identity)?
            .ok_or("publication request disappeared")?;
        check!(current.request.lease_epoch > self.request.request.lease_epoch);
        let mut expected = self.request.request.clone();
        expected.lease_epoch = current.request.lease_epoch;
        check!(eq; current.request, expected);
        check!(matches!(
            self.store.publication_lease_renew(
                &self.request.request.identity,
                self.request.request.lease_epoch,
            ),
            Err(census_store::StoreError::LeaseMismatch { .. })
        ));
        Ok(current)
    }
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

#[test]
fn expired_requested_stage_reloads_rebound_epoch_and_publishes() -> TestResult {
    let f = Retained::new()?;
    let stage = Stage::begin(&f.dataset, &f.store, &f.options)?;
    let rebound = f.rebound()?;
    check!(eq; rebound.state, State::Requested);
    f.unchanged_previous()?;
    let evidence = f.store.evidence_generation();
    let derived = f.store.derived_generation();
    let censuses = crate::workbook::Censuses::of(&f.dataset, &f.store.out_dir(), &[]);
    let published = stage.publish(&f.dataset, &f.options, |directory| {
        crate::workbook::write_artifacts(directory, &f.dataset, &f.options, &censuses)
    })?;
    check!(eq; current_workbook(&f.root)?, published);
    check!(eq; std::fs::read(&f.old)?, f.old_bytes);
    verify_published(&f.old)?;
    verify_published(&published)?;
    assert_no_temporaries(&f.root)?;
    check!(eq; f.store.evidence_generation(), evidence);
    check!(eq; f.store.derived_generation(), derived);
    let retained = f.rebound()?;
    check!(matches!(retained.state, State::Published { .. }));
    check!(eq; retained.request.lease_epoch, rebound.request.lease_epoch);
    check!(eq; f.store.publication_active(&retained.request.namespace)?, Some(retained.clone()));
    f.store.flush()?;
    let Retained {
        temp,
        store,
        root,
        old,
        old_bytes,
        ..
    } = f;
    drop(store);
    let reopened = Store::open(temp.path().join("store"))?;
    check!(eq; reopened.publication_read(&retained.request.identity)?, Some(retained.clone()));
    check!(eq; reopened.publication_active(&retained.request.namespace)?, Some(retained.clone()));
    check!(eq; reopened.publication_lease(&retained.request.identity)?, rebound.request.lease_epoch);
    check!(eq; current_workbook(&root)?, published);
    check!(eq; std::fs::read(&old)?, old_bytes);
    verify_published(&published)?;
    Ok(())
}

#[test]
fn rebound_requested_stage_still_refuses_changed_evidence_and_keeps_prior_bundle() -> TestResult {
    let f = Retained::new()?;
    let stage = Stage::begin(&f.dataset, &f.store, &f.options)?;
    let rebound = f.rebound()?;
    check!(eq; rebound.state, State::Requested);
    let censuses = crate::workbook::Censuses::of(&f.dataset, &f.store.out_dir(), &[]);
    let result = stage.publish(&f.dataset, &f.options, |directory| {
        crate::workbook::write_artifacts(directory, &f.dataset, &f.options, &censuses)?;
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Late School",
            "late-school",
            None,
        );
        f.store.append(Table::Schools, &school)?;
        Ok(())
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::InputFenceRefused { .. }
        ))
    ));
    f.unchanged_previous()?;
    check!(eq; f.rebound()?, rebound);
    assert_no_temporaries(&f.root)?;
    Ok(())
}
