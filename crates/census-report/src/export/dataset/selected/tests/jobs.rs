use super::{Fixture, TestResult};
use crate::export::{ExportDataset, ExportJobRequest};
use census_domain::model::{serialized_digest, ProfileSelectionV1};

#[tokio::test]
async fn selected_job_resumes_complete_pending_and_refuses_partial_or_corrupt_inputs() -> TestResult
{
    let fixture = Fixture::composed(false).await?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let job = "selected-durable-recovery-v1";
    let dataset = ExportDataset::for_job_request(&fixture.store, job, &request)?;
    let expected = dataset.archive_digest()?;
    drop(dataset);
    let directory = fixture.store.out_dir().join("export-inputs");
    let digest = serialized_digest(&job)?;
    let root = directory.join(format!("{digest}.selected-v1"));
    let pending = root.with_extension("selected-v1.pending");
    std::fs::rename(&root, &pending)?;
    let commitment_path = pending.join("commitment.json");
    let commitment = std::fs::read(&commitment_path)?;
    std::fs::remove_file(&commitment_path)?;
    check!(ExportDataset::for_job_request(&fixture.store, job, &request).is_err());
    check!(pending.join("input.json").is_file());
    check!(!root.exists());
    std::fs::write(&commitment_path, &commitment)?;
    check!(ExportDataset::resume_job_request(&fixture.store, job, &request).is_err());
    check!(pending.is_dir());
    check!(!root.exists());
    let resumed = ExportDataset::for_job_request(&fixture.store, job, &request)?;
    check!(eq; resumed.archive_digest()?, expected);
    check!(root.is_dir());
    check!(!pending.exists());
    drop(resumed);
    check!(eq; ExportDataset::resume_job_request(&fixture.store, job, &request)?.archive_digest()?, expected);
    let legacy = directory.join(format!("{digest}.json"));
    std::fs::write(&legacy, b"retained-conflicting-representation")?;
    check!(ExportDataset::resume_job_request(&fixture.store, job, &request).is_err());
    check!(eq; std::fs::read(&legacy)?, b"retained-conflicting-representation");
    std::fs::remove_file(&legacy)?;
    let proof = std::fs::read_dir(root.join("input.json.proofs"))?
        .next()
        .ok_or("proof object")??
        .path();
    let original = std::fs::read(&proof)?;
    let file = std::fs::OpenOptions::new().write(true).open(&proof)?;
    file.set_len(8_388_609)?;
    drop(file);
    check!(ExportDataset::resume_job_request(&fixture.store, job, &request).is_err());
    check!(eq; std::fs::metadata(&proof)?.len(), 8_388_609);
    check!(eq; std::fs::read(root.join("commitment.json"))?, commitment);
    std::fs::write(&proof, &original)?;
    check!(eq; ExportDataset::resume_job_request(&fixture.store, job, &request)?.archive_digest()?, expected);
    Ok(())
}

#[tokio::test]
async fn selected_job_resume_refuses_missing_input_without_capturing() -> TestResult {
    let fixture = Fixture::composed(false).await?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let job = "missing-native-workbook-input";
    let sequence = fixture.store.snapshot().sequence();
    let error = ExportDataset::resume_job_request(&fixture.store, job, &request)
        .err()
        .ok_or("resume unexpectedly captured a new input")?;
    check!(error
        .to_string()
        .contains("requires an existing frozen input"));
    let root = fixture
        .store
        .out_dir()
        .join("export-inputs")
        .join(format!("{}.selected-v1", serialized_digest(&job)?));
    check!(!root.exists());
    check!(!root.with_extension("selected-v1.pending").exists());
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn selected_job_refuses_legacy_retained_freeze_even_without_legacy_json() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path().join("store"))?;
    let job = "legacy-retained-input-v2";
    let root = crate::export::freeze_v2(&store, job)?;
    let path = root.directory().to_path_buf();
    let profile = super::fixture::profile()?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(store.store_identity()?, profile.id()?),
    };
    check!(ExportDataset::for_job_request(&store, job, &request).is_err());
    check!(path.is_dir());
    check!(ExportDataset::for_job_request(&store, job, &request).is_err());
    Ok(())
}

#[test]
fn selected_job_wire_requires_explicit_supported_contract() -> TestResult {
    for value in [
        r#"{}"#,
        r#"{"contract":"legacy_v2"}"#,
        r#"{"contract":"selected_publication_v1"}"#,
        r#"{"contract":"selected_publication_v2"}"#,
    ] {
        check!(serde_json::from_str::<ExportJobRequest>(value).is_err());
    }
    Ok(())
}
