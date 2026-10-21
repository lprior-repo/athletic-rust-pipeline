use super::refused;
use crate::acquisition::CaptureError;
use crate::StoreResult;
use census_domain::model::AcquisitionDigest;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const MAX_BYTES: usize = crate::records::codec::MAX_RECORD_BYTES;

pub(crate) fn encode<T: Serialize + ?Sized>(value: &T) -> StoreResult<Vec<u8>> {
    crate::records::codec::encode_canonical(value)
}

pub(crate) fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> StoreResult<T> {
    if bytes.len() > MAX_BYTES {
        return Err(refused("public resource record byte limit"));
    }
    let value: T =
        serde_json::from_slice(bytes).map_err(|_| refused("public resource record decoding"))?;
    if encode(&value)? != bytes {
        return Err(refused("public resource record is not canonical"));
    }
    Ok(value)
}

pub(crate) fn digest(bytes: &[u8]) -> StoreResult<AcquisitionDigest> {
    AcquisitionDigest::parse(&format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| CaptureError::Domain(error).into())
}

pub(crate) fn object_digest<T: Serialize + ?Sized>(value: &T) -> StoreResult<AcquisitionDigest> {
    digest(&encode(value)?)
}
