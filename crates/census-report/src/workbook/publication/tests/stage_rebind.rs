use super::*;
use crate::export::{ExportDataset, ExportJobRequest};
use crate::workbook::{self, Censuses, Options};
use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_store::{
    publish::{Digest, Intent, Request, State},
    Table,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture_options(out: Option<std::path::PathBuf>) -> TestResult<Options> {
    Ok(Options {
        address_corpus: None,
        grad_year: Some(2027),
        out,
        limit: None,
        scope: crate::report::Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
    })
}

struct Retained {
    fixture: crate::export::selected_test_fixture::Fixture,
    request: Intent,
    old: PathBuf,
    old_bytes: Vec<u8>,
    root: PathBuf,
}

impl Retained {
    async fn new(job: &str) -> TestResult<Self> {
        let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
        let objects = tempfile::tempdir()?;
        let dataset = fixture.capture(objects.path())?;
        let options = fixture_options(None)?;
        let root = fixture.store.out_dir().join("publication");
        let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
        let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
        let old_bytes = std::fs::read(&old)?;
        let job_request = ExportJobRequest::SelectedPublicationV1 {
            selection: fixture.selection.clone(),
        };
        let request = expiring_requested(&fixture, &root, job, &job_request).await?;
        Ok(Self {
            fixture,
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
            .fixture
            .store
            .publication_read(&self.request.request.identity)?
            .ok_or("publication request disappeared")?;
        check!(current.request.lease_epoch > self.request.request.lease_epoch);
        let mut expected = self.request.request.clone();
        expected.lease_epoch = current.request.lease_epoch;
        check!(eq; current.request, expected);
        check!(matches!(
            self.fixture.store.publication_lease_renew(
                &self.request.request.identity,
                self.request.request.lease_epoch,
            ),
            Err(census_store::StoreError::LeaseMismatch { .. })
        ));
        Ok(current)
    }
}

async fn expiring_requested(
    fixture: &crate::export::selected_test_fixture::Fixture,
    root: &Path,
    job: &str,
    job_request: &ExportJobRequest,
) -> TestResult<Intent> {
    let dataset = ExportDataset::for_job_request(&fixture.store, job, job_request)?;
    let options = fixture_options(None)?;
    let namespace = binding::namespace(root)?;
    let input = binding::input(&dataset, &fixture.store, &options)?;
    let identity =
        binding::digest(&(&namespace, &dataset.lineage.input_generation, &input.recipe))?;
    let expires = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("lease expiry overflow")?
        .to_rfc3339();
    let lease_epoch =
        fixture
            .store
            .lease_acquire(identity.as_str(), "workbook-publication", &expires)?;
    let request = fixture.store.publication_request(Request {
        namespace,
        identity,
        expected: pointer::read(root)?,
        input,
        lease_epoch,
    })?;
    await_expired(&fixture.store, &request.request.identity)?;
    Ok(request)
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

#[tokio::test]
async fn expired_requested_stage_reloads_rebound_epoch_and_publishes() -> TestResult {
    let f = Retained::new("requested-stage-rebind").await?;
    let options = fixture_options(None)?;
    let job_request = ExportJobRequest::SelectedPublicationV1 {
        selection: f.fixture.selection.clone(),
    };
    let job_dataset =
        ExportDataset::for_job_request(&f.fixture.store, "requested-stage-rebind", &job_request)?;
    let stage = Stage::begin(&job_dataset, &f.fixture.store, &options)?;
    let rebound = f.rebound()?;
    check!(eq; rebound.state, State::Requested);
    f.unchanged_previous()?;
    let evidence = f.fixture.store.evidence_generation();
    let derived = f.fixture.store.derived_generation();
    let censuses = Censuses::of(
        &job_dataset,
        &f.fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let published = stage.publish(&job_dataset, &options, |directory| {
        workbook::write_artifacts(directory, &job_dataset, &options, &censuses)
    })?;
    check!(eq; current_workbook(&f.root)?, published);
    check!(eq; std::fs::read(&f.old)?, f.old_bytes);
    verify_published(&f.old)?;
    verify_published(&published)?;
    assert_no_temporaries(&f.root)?;
    check!(eq; f.fixture.store.evidence_generation(), evidence);
    check!(eq; f.fixture.store.derived_generation(), derived);
    let retained = f.rebound()?;
    check!(matches!(retained.state, State::Published { .. }));
    check!(eq; retained.request.lease_epoch, rebound.request.lease_epoch);
    check!(eq; f.fixture.store.publication_active(&retained.request.namespace)?, Some(retained.clone()));
    f.fixture.store.flush()?;
    let temp = f.fixture.directory.path().to_owned();
    let root = f.root.clone();
    let old = f.old.clone();
    let old_bytes = f.old_bytes.clone();
    let retained_identity = retained.request.identity.clone();
    let retained_namespace = retained.request.namespace.clone();
    let retained_epoch = retained.request.lease_epoch;
    let Retained {
        fixture:
            crate::export::selected_test_fixture::Fixture {
                directory: _keep,
                store,
                ..
            },
        ..
    } = f;
    drop(store);
    let (reopened, _) =
        Store::open_public_identity_qualification_v3(temp.join("store"))?.into_parts();
    check!(eq; reopened.publication_read(&retained_identity)?, Some(retained.clone()));
    check!(eq; reopened.publication_active(&retained_namespace)?, Some(retained.clone()));
    check!(eq; reopened.publication_lease(&retained_identity)?, retained_epoch);
    check!(eq; current_workbook(&root)?, published);
    check!(eq; std::fs::read(&old)?, old_bytes);
    verify_published(&published)?;
    Ok(())
}

#[tokio::test]
async fn rebound_requested_stage_still_refuses_changed_evidence_and_keeps_prior_bundle(
) -> TestResult {
    let f = Retained::new("rebound-evidence-changed").await?;
    let options = fixture_options(None)?;
    let job_request = ExportJobRequest::SelectedPublicationV1 {
        selection: f.fixture.selection.clone(),
    };
    let job_dataset =
        ExportDataset::for_job_request(&f.fixture.store, "rebound-evidence-changed", &job_request)?;
    let stage = Stage::begin(&job_dataset, &f.fixture.store, &options)?;
    let rebound = f.rebound()?;
    check!(eq; rebound.state, State::Requested);
    let censuses = Censuses::of(
        &job_dataset,
        &f.fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let result = stage.publish(&job_dataset, &options, |directory| {
        let counts = workbook::write_artifacts(directory, &job_dataset, &options, &censuses)?;
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Late School",
            "late-school",
            None,
        );
        f.fixture.store.append(Table::Schools, &school)?;
        Ok(counts)
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
