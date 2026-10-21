use super::*;

pub(super) fn validate_manifest(
    declaration: &CaptureQualification,
    facts: &WorkFacts,
    manifest: Option<&(RunManifest, AcquisitionDigest)>,
) -> StoreResult<()> {
    let (manifest, manifest_digest) = manifest.ok_or(CaptureError::CaptureRunManifestMissing)?;
    if manifest.store_identity != declaration.store_identity().as_str()
        || facts.work.run() != manifest.run
        || facts.work.run_manifest_digest() != manifest_digest
        || manifest.cohort.get() != 2027
    {
        return Err(CaptureError::CaptureRunMismatch.into());
    }
    if facts.work.slot().store_identity() != declaration.store_identity() {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    Ok(())
}
pub(super) fn require_supported_work(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    if work.slot().public_resource().is_some() {
        return crate::acquisition::public_resources::read::validate_work(
            snapshot, meta, journal, receipts, work, budget,
        );
    }
    Ok(())
}
