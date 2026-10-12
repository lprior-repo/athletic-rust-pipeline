use super::*;
use census_store::publish::{Digest, Intent, Request, Root, State};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    root: PathBuf,
    old: PathBuf,
    old_bytes: Vec<u8>,
    desired: Digest,
    later: Digest,
    intent: Intent,
}
impl Fixture {
    fn new() -> TestResult<Self> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path().join("store"))?;
        let options = super::fence_tests::options(None)?;
        let old = crate::workbook::build(&store, &options)?;
        let old_bytes = std::fs::read(&old)?;
        let root = store.out_dir().join("publication");
        let mut desired_options = options.clone();
        desired_options.limit = Some(1);
        let dataset = ExportDataset::load(&store)?;
        let desired = install_bundle(
            &store,
            &root,
            &temp.path().join("desired"),
            &desired_options,
        )?;
        let mut later_options = options;
        later_options.limit = Some(2);
        let later = install_bundle(&store, &root, &temp.path().join("later"), &later_options)?;
        let namespace = binding::namespace(&root)?;
        let identity = binding::digest(&("matrix", &namespace))?;
        let epoch = store.publication_lease(&identity)?;
        let request = store.publication_request(Request {
            namespace,
            identity,
            expected: pointer::read(&root)?,
            input: binding::input(&dataset, &store, &desired_options)?,
            lease_epoch: epoch,
        })?;
        let intent = store
            .publication_fence(&request.request, &binding::run_manifest(&store)?)?
            .begin(&request, &desired)?;
        Ok(Self {
            _temp: temp,
            store,
            root,
            old,
            old_bytes,
            desired,
            later,
            intent,
        })
    }
    fn selected(&self, digest: &Digest) -> TestResult {
        pointer::switch(&self.root, digest)?;
        sync_directory(&self.root)?;
        Ok(())
    }
    fn preserved(&self) -> TestResult {
        check!(eq; std::fs::read(&self.old)?, self.old_bytes);
        verify_published(&self.old)?;
        Ok(())
    }
    fn published(&self) -> TestResult {
        self.selected(&self.desired)?;
        self.store.publication_mark(
            &self.intent,
            State::Published {
                receipt: self.desired.clone(),
            },
        )?;
        Ok(())
    }
}
fn install_bundle(
    store: &Store,
    root: &Path,
    other: &Path,
    options: &Options,
) -> TestResult<Digest> {
    let mut options = options.clone();
    options.out = Some(other.into());
    let workbook = crate::workbook::build(store, &options)?;
    let dir = workbook.parent().ok_or("bundle directory")?;
    let digest = Digest::parse(
        dir.file_name()
            .and_then(|name| name.to_str())
            .ok_or("digest")?,
    )?;
    std::fs::rename(dir, root.join("generations").join(digest.as_str()))?;
    sync_directory(&root.join("generations"))?;
    Ok(digest)
}

#[test]
fn publication_recovery_desired_selected_records_success_without_rollback() -> TestResult {
    let f = Fixture::new()?;
    f.selected(&f.desired)?;
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::DesiredPublished { bundle: f.desired.clone() });
    check!(eq; pointer::read(&f.root)?, Root::Bundle(f.desired.clone()));
    check!(eq; f.store.publication_read(&f.intent.request.identity)?.ok_or("intent")?.state, State::Published { receipt: f.desired.clone() });
    f.preserved()
}
#[test]
fn publication_recovery_expected_selected_retries_the_same_intent() -> TestResult {
    let f = Fixture::new()?;
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::ExpectedRetried { bundle: f.desired.clone() });
    check!(eq; pointer::read(&f.root)?, Root::Bundle(f.desired.clone()));
    check!(eq; f.store.publication_read(&f.intent.request.identity)?.ok_or("intent")?.request, f.intent.request);
    f.preserved()
}
#[test]
fn publication_recovery_another_valid_root_is_never_overwritten() -> TestResult {
    let f = Fixture::new()?;
    f.selected(&f.later)?;
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::AnotherRootSelected { selected: f.later.clone(), historical: f.desired.clone() });
    check!(eq; pointer::read(&f.root)?, Root::Bundle(f.later.clone()));
    check!(eq; f.store.publication_read(&f.intent.request.identity)?.ok_or("intent")?.state, State::Superseded { desired: f.desired.clone() });
    f.preserved()
}
#[test]
fn publication_recovery_corrupt_pointer_requires_explicit_policy() -> TestResult {
    let f = Fixture::new()?;
    let bad = f.root.join("bad.tmp");
    std::os::unix::fs::symlink("../escape", &bad)?;
    std::fs::rename(bad, f.root.join("current"))?;
    match recover_publication(&f.store, &f.root) {
        Err(ReportError::Publication(PublicationError::RecoveryRequired {
            retained,
            detail,
            ..
        })) => {
            check!(eq; retained, vec![f.intent.request.expected.clone(), Root::Bundle(f.desired.clone())]);
            check!(detail.contains("owned relative link"));
        }
        other => return Err(format!("wrong recovery outcome: {other:?}").into()),
    }
    check!(eq; std::fs::read_link(f.root.join("current"))?, PathBuf::from("../escape"));
    f.preserved()
}
#[test]
fn publication_recovery_published_receipt_with_later_root_is_historical_success() -> TestResult {
    let f = Fixture::new()?;
    f.published()?;
    f.selected(&f.later)?;
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::HistoricalSuccess { bundle: f.desired.clone() });
    check!(eq; pointer::read(&f.root)?, Root::Bundle(f.later.clone()));
    f.preserved()
}
#[test]
fn publication_recovery_published_corrupt_bundle_blocks_sealing_and_preserves_evidence(
) -> TestResult {
    let f = Fixture::new()?;
    f.published()?;
    f.selected(&f.later)?;
    let corrupt = f
        .root
        .join("generations")
        .join(f.desired.as_str())
        .join("recruiting.csv");
    std::fs::write(&corrupt, "corrupt")?;
    check!(
        matches!(recover_publication(&f.store, &f.root), Err(ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. })) if bundle == f.desired)
    );
    check!(eq; std::fs::read(&corrupt)?, b"corrupt");
    check!(eq; pointer::read(&f.root)?, Root::Bundle(f.later.clone()));
    let workbook = current_workbook(&f.root)?;
    let lineage = crate::export::store_lineage(&f.store)?;
    check!(
        matches!(open_publication(&workbook, &lineage, crate::report::Scope::AllSources, census_domain::model::GradYear::CO2027), Err(ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. })) if bundle == f.desired)
    );
    f.preserved()
}
#[test]
fn publication_recovery_expected_selected_stale_lease_is_superseded() -> TestResult {
    let f = Fixture::new()?;
    f.store.lease_release(
        f.intent.request.identity.as_str(),
        f.intent.request.lease_epoch,
    )?;
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::ExpectedSuperseded { bundle: f.desired.clone() });
    check!(eq; pointer::read(&f.root)?, f.intent.request.expected);
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::ExpectedSuperseded { bundle: f.desired.clone() });
    check!(eq; pointer::read(&f.root)?, f.intent.request.expected);
    f.preserved()
}
#[test]
fn publication_recovery_missing_pointer_never_guesses_a_retained_root() -> TestResult {
    let f = Fixture::new()?;
    std::fs::remove_file(f.root.join("current"))?;
    check!(
        matches!(recover_publication(&f.store, &f.root), Err(ReportError::Publication(PublicationError::RecoveryRequired { retained, .. })) if retained == vec![f.intent.request.expected.clone(), Root::Bundle(f.desired.clone())])
    );
    check!(eq; pointer::read(&f.root)?, Root::Absent);
    f.preserved()
}

#[test]
fn publication_fence_visible_rename_fsync_failure_retains_intent() -> TestResult {
    let f = Fixture::new()?;
    pointer::switch(&f.root, &f.desired)?;
    let displaced = f._temp.path().join("displaced-publication");
    std::fs::rename(&f.root, &displaced)?;
    let result = pointer::finish(&f.root, &f.intent, || {
        f.store.publication_mark(
            &f.intent,
            State::Published {
                receipt: f.desired.clone(),
            },
        )?;
        Ok(())
    });
    check!(
        matches!(result, Err(ReportError::Publication(PublicationError::PublicationDurabilityUncertain { ref bundle, .. })) if bundle == &f.desired)
    );
    check!(eq; pointer::read(&displaced)?, Root::Bundle(f.desired.clone()));
    check!(eq; f.store.publication_read(&f.intent.request.identity)?, Some(f.intent.clone()));
    std::fs::rename(&displaced, &f.root)?;
    f.preserved()?;
    check!(eq; recover_publication(&f.store, &f.root)?, RecoveryOutcome::DesiredPublished { bundle: f.desired.clone() });
    f.preserved()
}

#[test]
fn publication_integrity_incident_blocks_opening_an_otherwise_valid_seal_generation() -> TestResult
{
    let f = Fixture::new()?;
    f.published()?;
    f.selected(&f.later)?;
    let corrupt = f
        .root
        .join("generations")
        .join(f.desired.as_str())
        .join("recruiting.csv");
    std::fs::write(&corrupt, "corrupt")?;
    check!(matches!(
        recover_publication(&f.store, &f.root),
        Err(ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. }))
            if bundle == f.desired
    ));
    let lineage = crate::export::store_lineage(&f.store)?;
    let error = match open_publication(
        &f.old,
        &lineage,
        crate::report::Scope::AllSources,
        census_domain::model::GradYear::CO2027,
    ) {
        Err(error) => error,
        Ok(_) => return Err("seal opening bypassed the retained integrity incident".into()),
    };
    check!(matches!(
        error,
        ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. })
            if bundle == f.desired
    ));
    f.preserved()
}
