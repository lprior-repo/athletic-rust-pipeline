use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Deserializer, Serializer};

pub(super) fn serialize<S: Serializer>(body: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(body))
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<u8>, D::Error> {
    let text = String::deserialize(parser)?;
    let maximum = super::MAX_BODY.div_ceil(3).saturating_mul(4);
    if text.len() > maximum {
        return Err(serde::de::Error::custom("public school intake body limit"));
    }
    let body = STANDARD.decode(&text).map_err(serde::de::Error::custom)?;
    if body.len() > super::MAX_BODY || STANDARD.encode(&body) != text {
        return Err(serde::de::Error::custom(
            "public school intake canonical body",
        ));
    }
    Ok(body)
}
