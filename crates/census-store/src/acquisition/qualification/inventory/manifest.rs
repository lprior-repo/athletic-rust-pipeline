use super::{
    digest, AcquisitionDigest, AcquisitionReadBudget, CaptureError, Keyspace, Readable,
    RunManifest, Snapshot, StoreError, StoreResult, MAX_CAPTURE_RECORD_BYTES,
};

pub(super) fn load(
    snapshot: &Snapshot,
    meta: &Keyspace,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<(RunManifest, AcquisitionDigest)>> {
    if let Some(length) = snapshot
        .size_of(meta, crate::run::RUN_MANIFEST)
        .map_err(|source| StoreError::Read { source })?
    {
        let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        if length > MAX_CAPTURE_RECORD_BYTES {
            return Err(CaptureError::RecordTooLarge.into());
        }
        budget.encoded(
            length
                .checked_add(crate::run::RUN_MANIFEST.len())
                .ok_or(StoreError::CounterOverflow)?,
        )?;
    }
    crate::run::read(snapshot, meta)?
        .map(|manifest| {
            let digest = digest(&manifest)?;
            Ok((manifest, digest))
        })
        .transpose()
}
