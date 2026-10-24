use super::{Fixture, TestResult};
use crate::export::{ExportDataset, ExportJobRequest};
use census_domain::model::serialized_digest;
use std::path::PathBuf;

mod proofs;

#[tokio::test]
async fn explicit_job_recovery_reuses_empty_staging_then_replays_the_same_committed_input(
) -> TestResult {
    let fixture = Fixture::composed(false).await?;
    let request = request(&fixture);
    let job = "workbook:failed-empty-physical-preflight";
    let root = root(&fixture, job)?;
    let pending = root.with_extension("selected-v1.pending");
    std::fs::create_dir_all(pending.join("input.json.proofs"))?;
    let sequence = fixture.store.snapshot().sequence();
    check!(ExportDataset::for_job_request(&fixture.store, job, &request).is_err());
    check!(ExportDataset::resume_job_request(&fixture.store, job, &request).is_err());
    let dataset = ExportDataset::recover_job_request(&fixture.store, job, &request)?;
    check!(eq; dataset.lineage.export_job.as_deref(), Some(job));
    let digest = dataset.archive_digest()?;
    drop(dataset);
    check!(root.is_dir());
    check!(!pending.exists());
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    check!(eq; ExportDataset::recover_job_request(&fixture.store, job, &request)?.archive_digest()?, digest);
    std::fs::rename(&root, &pending)?;
    check!(eq; ExportDataset::recover_job_request(&fixture.store, job, &request)?.archive_digest()?, digest);
    check!(eq; ExportDataset::resume_job_request(&fixture.store, job, &request)?.archive_digest()?, digest);
    std::fs::create_dir_all(pending.join("input.json.proofs"))?;
    check!(ExportDataset::recover_job_request(&fixture.store, job, &request).is_err());
    check!(root.join("commitment.json").is_file());
    check!(pending.join("input.json.proofs").is_dir());
    Ok(())
}

#[tokio::test]
async fn explicit_job_recovery_requires_an_existing_job_and_preserves_partial_artifacts(
) -> TestResult {
    let fixture = Fixture::composed(false).await?;
    let request = request(&fixture);
    let missing = "workbook:missing-recovery";
    let sequence = fixture.store.snapshot().sequence();
    check!(ExportDataset::recover_job_request(&fixture.store, missing, &request).is_err());
    check!(!root(&fixture, missing)?.exists());
    check!(!root(&fixture, missing)?
        .with_extension("selected-v1.pending")
        .exists());
    for (job, artifact) in [
        ("workbook:partial-archive", "input.json"),
        ("workbook:retained-proof", "input.json.proofs/retained"),
        ("workbook:unexpected-staging", "unexpected"),
    ] {
        let root = root(&fixture, job)?;
        let pending = root.with_extension("selected-v1.pending");
        std::fs::create_dir_all(pending.join("input.json.proofs"))?;
        let path = pending.join(artifact);
        std::fs::write(&path, b"preserve-uncommitted-evidence")?;
        let before = retained_staging(&pending)?;
        let error = ExportDataset::recover_job_request(&fixture.store, job, &request)
            .err()
            .ok_or("partial artifacts unexpectedly captured")?;
        let expected = if artifact == "input.json.proofs/retained" {
            "selected proof inventory contains an invalid regular object"
        } else {
            "uncommitted selected staging is not exactly proof-only storage"
        };
        check!(
            matches!(error, crate::report::ReportError::Invariant { detail } if detail == expected)
        );
        check!(eq; retained_staging(&pending)?, before);
        check!(eq; std::fs::read(&path)?, b"preserve-uncommitted-evidence");
        check!(!root.exists());
        check!(!pending.join("commitment.json").exists());
    }
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn explicit_job_recovery_refuses_symlink_staging_and_never_touches_its_target() -> TestResult
{
    let fixture = Fixture::composed(false).await?;
    let request = request(&fixture);
    let target = tempfile::tempdir()?;
    std::fs::write(target.path().join("preserve"), b"external-evidence")?;
    for (job, link_root) in [
        ("workbook:linked-pending", true),
        ("workbook:linked-proofs", false),
    ] {
        let root = root(&fixture, job)?;
        let pending = root.with_extension("selected-v1.pending");
        std::fs::create_dir_all(pending.parent().ok_or("pending parent")?)?;
        let link = if link_root {
            pending.clone()
        } else {
            std::fs::create_dir(&pending)?;
            pending.join("input.json.proofs")
        };
        std::os::unix::fs::symlink(target.path(), &link)?;
        check!(ExportDataset::recover_job_request(&fixture.store, job, &request).is_err());
        check!(std::fs::symlink_metadata(&link)?.file_type().is_symlink());
        check!(eq; std::fs::read(target.path().join("preserve"))?, b"external-evidence");
        check!(!root.exists());
    }
    Ok(())
}

fn request(fixture: &Fixture) -> ExportJobRequest {
    ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    }
}

#[tokio::test]
async fn explicit_job_recovery_keeps_the_committed_selection_and_archive_unchanged() -> TestResult {
    use census_domain::model::{ProfileSelectionV1, ProvisionalInclusionV1, QueryProfileV1};
    let fixture = Fixture::composed(false).await?;
    let job = "workbook:committed-recovery-binding";
    let dataset = ExportDataset::for_job_request(&fixture.store, job, &request(&fixture))?;
    drop(dataset);
    let root = root(&fixture, job)?;
    let archive = std::fs::read(root.join("input.json"))?;
    let commitment = std::fs::read(root.join("commitment.json"))?;
    let mut input = super::fixture::profile()?.input().clone();
    input.include_provisional = ProvisionalInclusionV1::Include;
    let profile = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&profile)?;
    let changed = ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?),
    };
    let sequence = fixture.store.snapshot().sequence();
    let error = ExportDataset::recover_job_request(&fixture.store, job, &changed)
        .err()
        .ok_or("recovery changed committed selection")?;
    check!(
        matches!(error, crate::report::ReportError::Invariant { ref detail }
        if detail == "selected export job request, identity or store mismatch")
    );
    check!(eq; std::fs::read(root.join("input.json"))?, archive);
    check!(eq; std::fs::read(root.join("commitment.json"))?, commitment);
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    Ok(())
}

fn root(fixture: &Fixture, job: &str) -> TestResult<PathBuf> {
    Ok(fixture
        .store
        .out_dir()
        .join("export-inputs")
        .join(format!("{}.selected-v1", serialized_digest(&job)?)))
}

#[tokio::test]
async fn commitment_only_staging_refuses_before_capturing_and_preserves_all_bytes() -> TestResult {
    let fixture = Fixture::composed(false).await?;
    let job = "workbook:commitment-only-recovery";
    let root = root(&fixture, job)?;
    let pending = root.with_extension("selected-v1.pending");
    let objects = pending.join("input.json.proofs");
    std::fs::create_dir_all(&objects)?;
    let retained = b"retained-proof-before-partial-commitment";
    let digest = crate::export::proof_objects::install(&objects, retained)?;
    let commitment = pending.join("commitment.json");
    std::fs::write(&commitment, b"preserve-malformed-commitment")?;
    let sequence = fixture.store.snapshot().sequence();
    let error = ExportDataset::recover_job_request(&fixture.store, job, &request(&fixture))
        .err()
        .ok_or("commitment-only staging unexpectedly captured")?;
    check!(
        matches!(error, crate::report::ReportError::Decode { ref path, .. } if path == &commitment)
    );
    check!(eq; std::fs::read(objects.join(digest))?, retained);
    check!(eq; std::fs::read(commitment)?, b"preserve-malformed-commitment");
    check!(!pending.join("input.json").exists());
    check!(!root.exists());
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    Ok(())
}

fn retained_staging(
    root: &std::path::Path,
) -> TestResult<std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>> {
    let mut result = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            result.insert(path.clone(), None);
            for child in std::fs::read_dir(&path)? {
                let child = child?;
                check!(child.file_type()?.is_file());
                result.insert(child.path(), Some(std::fs::read(child.path())?));
            }
        } else {
            check!(entry.file_type()?.is_file());
            result.insert(path.clone(), Some(std::fs::read(path)?));
        }
    }
    Ok(result)
}
