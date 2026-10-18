use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub(in super::super) fn serialize<S: serde::Serializer>(
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let raw: &RawValue = serde_json::from_slice(bytes).map_err(serde::ser::Error::custom)?;
    raw.serialize(serializer)
}

pub(in super::super) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error> {
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    let value = raw.get().as_bytes();
    if value.len() > census_domain::result_school_correspondence::MAX_RESULT_SCHOOL_RECORD_BYTES {
        return Err(serde::de::Error::custom("proof JSON byte ceiling"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(value.len())
        .map_err(serde::de::Error::custom)?;
    bytes.extend_from_slice(value);
    Ok(bytes)
}
