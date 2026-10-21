use super::*;
use crate::export::{ExportDataset, ExportJobRequest};
use crate::workbook::{self, Censuses, Options};
use census_domain::UsJurisdiction;
use census_store::publish::{Digest, Intent, Request, Root, State};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture_options(out: Option<std::path::PathBuf>) -> TestResult<Options> {
    Ok(Options {
        grad_year: Some(2027),
        out,
        limit: None,
        scope: crate::report::Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
    })
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
    let intent = fixture.store.publication_request(Request {
        namespace,
        identity,
        expected: pointer::read(root)?,
        input,
        lease_epoch,
    })?;
    await_expired(&fixture.store, &intent.request.identity)?;
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

async fn build_other_bundle(
    fixture: &crate::export::selected_test_fixture::Fixture,
    other_root: PathBuf,
) -> TestResult<PathBuf> {
    let other_objects = tempfile::tempdir()?;
    let other_dataset = fixture.capture(other_objects.path())?;
    let other = fixture_options(Some(other_root))?;
    let censuses = Censuses::of(
        &other_dataset,
        &fixture.store.out_dir(),
        &other.jurisdictions,
    )?;
    let workbook = workbook::build_from(&other_dataset, &fixture.store, &other, &censuses)?;
    let generation = workbook.parent().ok_or("other generation missing")?;
    Ok(generation.to_path_buf())
}

#[tokio::test]
async fn expired_requested_stage_cannot_replace_unconsumed_integrity_history() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let options = fixture_options(None)?;
    let root = fixture.store.out_dir().join("publication");
    let other_root = tempfile::tempdir()?.keep();
    let other_dir = build_other_bundle(&fixture, other_root).await?;
    let other_name = other_dir.file_name().ok_or("other digest missing")?;
    let other_digest = Digest::parse(other_name.to_str().ok_or("other digest is not UTF-8")?)?;
    fixture.append_second_meet().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture
        .capture(objects.path())
        .map_err(|error| format!("capturing updated selected input: {error}"))?;
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)
        .map_err(|error| format!("deriving updated selected censuses: {error}"))?;
    let old = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let namespace = binding::namespace(&root)?;
    let active = fixture
        .store
        .publication_active(&namespace)?
        .ok_or("active publication missing")?;
    let active_bundle = active.desired().ok_or("active bundle missing")?.clone();
    check!(ne; other_digest, active_bundle);
    let selected = root.join("generations").join(other_name);
    std::fs::rename(&other_dir, &selected)?;
    pointer::switch(&root, &other_digest)?;
    sync_directory(&root)?;
    let selected = selected.join("workbook.xlsx");
    let selected_bytes = std::fs::read(&selected)?;
    let job_request = ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let request =
        expiring_requested(&fixture, &root, "requested-integrity-history", &job_request).await?;
    let lease = fixture
        .store
        .get_lease_record(request.request.identity.as_str())?;
    let corrupt = old
        .parent()
        .ok_or("old generation missing")?
        .join("recruiting.csv");
    std::fs::write(&corrupt, "retained corrupt artifact")?;
    let job_dataset = ExportDataset::for_job_request(
        &fixture.store,
        "requested-integrity-history",
        &job_request,
    )?;
    let error = match Stage::begin(&job_dataset, &fixture.store, &options) {
        Err(error) => error,
        Ok(_) => return Err("Requested rebind discarded an unconsumed integrity incident".into()),
    };
    check!(matches!(
        error,
        ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. })
            if bundle == active_bundle
    ));
    check!(eq; fixture.store.publication_active(&namespace)?, Some(active.clone()));
    check!(eq; fixture.store.publication_read(&request.request.identity)?, Some(request.clone()));
    check!(
        eq;
        fixture.store.get_lease_record(request.request.identity.as_str())?,
        lease
    );
    check!(eq; current_workbook(&root)?, selected);
    check!(eq; std::fs::read(&selected)?, selected_bytes);
    check!(eq; std::fs::read(&corrupt)?, b"retained corrupt artifact");
    check!(matches!(
        incident::check_root(&root),
        Err(ReportError::Publication(PublicationError::IntegrityIncident { bundle, .. }))
            if bundle == active_bundle
    ));
    assert_no_temporaries(&root)?;
    fixture.store.flush()?;
    let temp = fixture.directory.path().to_owned();
    let crate::export::selected_test_fixture::Fixture {
        directory: _keep,
        store,
        ..
    } = fixture;
    drop(store);
    let (reopened, _) =
        Store::open_public_identity_qualification_v3(temp.join("store"))?.into_parts();
    check!(eq; reopened.publication_active(&namespace)?, Some(active));
    check!(eq; reopened.publication_read(&request.request.identity)?, Some(request.clone()));
    check!(eq; reopened.get_lease_record(request.request.identity.as_str())?, lease);
    check!(eq; current_workbook(&root)?, selected);
    check!(eq; std::fs::read(&selected)?, selected_bytes);
    Ok(())
}

#[tokio::test]
async fn requested_stage_without_active_history_rebinds_exactly_once_before_publication(
) -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let root = fixture.store.out_dir().join("publication");
    let job_request = ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let request =
        expiring_requested(&fixture, &root, "requested-without-active", &job_request).await?;
    check!(eq; fixture.store.publication_active(&request.request.namespace)?, None);
    let job_dataset =
        ExportDataset::for_job_request(&fixture.store, "requested-without-active", &job_request)?;
    let stage = Stage::begin(&job_dataset, &fixture.store, &fixture_options(None)?)?;
    let rebound = fixture
        .store
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
    check!(eq; fixture.store.publication_active(&request.request.namespace)?, Some(rebound));
    check!(eq; pointer::read(&root)?, Root::Absent);
    let options = fixture_options(None)?;
    let censuses = Censuses::of(
        &job_dataset,
        &fixture.store.out_dir(),
        &options.jurisdictions,
    )?;
    let published = stage.publish(&job_dataset, &options, |directory| {
        workbook::write_artifacts(directory, &job_dataset, &options, &censuses)
    })?;
    let retained = fixture
        .store
        .publication_read(&request.request.identity)?
        .ok_or("published request missing")?;
    check!(eq; retained.request.lease_epoch, next_epoch);
    check!(matches!(retained.state, State::Published { .. }));
    check!(eq; fixture.store.publication_active(&request.request.namespace)?, Some(retained));
    check!(eq; current_workbook(&root)?, published);
    verify_published(&published)?;
    assert_no_temporaries(&root)?;
    Ok(())
}
