use super::*;
use sha2::{Digest, Sha256};

pub struct FrozenEvidenceSubjectAttachmentBundleV1 {
    pub(super) bytes: Vec<u8>,
    pub(super) reference: AttachmentObjectReferenceV1,
}

impl FrozenEvidenceSubjectAttachmentBundleV1 {
    pub fn from_owned_bytes(
        bytes: Vec<u8>,
        expected: AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = (|| {
            bound(
                "frozen attachment archive capacity",
                bytes.capacity(),
                MAX_ARCHIVE_BYTES,
            )?;
            require_reference(&bytes, &expected, grant)?;
            Ok(Self {
                bytes,
                reference: expected,
            })
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn reference(&self) -> &AttachmentObjectReferenceV1 {
        &self.reference
    }
}

pub(super) fn require_reference(
    bytes: &[u8],
    expected: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    steps(grant, 1)?;
    bound(
        "frozen attachment archive bytes",
        bytes.len(),
        MAX_ARCHIVE_BYTES,
    )?;
    if bytes.is_empty()
        || u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)? != expected.bytes
    {
        return Err(Error::Corrupt("frozen attachment archive length differs").into());
    }
    steps(grant, 1)?;
    if format!("{:x}", Sha256::digest(bytes)) != expected.digest.as_str() {
        return Err(Error::Corrupt("frozen attachment archive digest differs").into());
    }
    Ok(())
}
