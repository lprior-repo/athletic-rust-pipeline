use super::*;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

pub(super) fn encode(
    reference: &AttachmentObjectReferenceV1,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Material> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let bytes = bounded_bytes(reference)?;
    if bytes != raw.len() {
        return Err(Error::Corrupt("inline object length").into());
    }
    let encoded = bytes
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    grant.charge_bytes(encoded).map_err(Error::Domain)?;
    require_digest(reference, raw, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let mut data = String::new();
    data.try_reserve_exact(encoded)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    BASE64_STANDARD.encode_string(raw, &mut data);
    if data.len() != encoded {
        return Err(Error::Corrupt("inline object encoding length").into());
    }
    Ok(Material::Inline {
        reference: reference.clone(),
        data,
    })
}

pub(super) fn decode(
    reference: &AttachmentObjectReferenceV1,
    data: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let bytes = bounded_bytes(reference)?;
    let encoded = bytes
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    if data.len() != encoded {
        return Err(Error::Corrupt("inline object encoded length").into());
    }
    grant.charge_steps(2).map_err(Error::Domain)?;
    let mut raw = Vec::new();
    raw.try_reserve_exact(bytes)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    raw.resize(bytes, 0);
    let decoded = BASE64_STANDARD
        .decode_slice(data, &mut raw)
        .map_err(|_| Error::Corrupt("inline object base64"))?;
    if decoded != bytes {
        return Err(Error::Corrupt("inline object decoded length").into());
    }
    require_digest(reference, &raw, grant)?;
    Ok(raw)
}

fn require_digest(
    reference: &AttachmentObjectReferenceV1,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if format!("{:x}", Sha256::digest(raw)) != reference.digest.as_str() {
        return Err(Error::Corrupt("inline object digest").into());
    }
    Ok(())
}
