use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::StatusCode;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

pub mod status {
    use super::*;

    pub fn serialize<S: Serializer>(value: &StatusCode, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(value.as_u16())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<StatusCode, D::Error> {
        let raw = u16::deserialize(deserializer)?;
        StatusCode::from_u16(raw).map_err(D::Error::custom)
    }
}

pub mod headers {
    use super::*;

    pub fn serialize<S: Serializer>(value: &HeaderMap, serializer: S) -> Result<S::Ok, S::Error> {
        let mut pairs = Vec::with_capacity(value.len());
        for (name, value) in value {
            let value = value.to_str().map_err(serde::ser::Error::custom)?;
            pairs.push((name.as_str(), value));
        }
        pairs.sort_by(|left, right| left.0.cmp(right.0));
        pairs.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<HeaderMap, D::Error> {
        let pairs = Vec::<(String, String)>::deserialize(deserializer)?;
        let mut headers = HeaderMap::with_capacity(pairs.len());
        for (name, value) in pairs {
            let name = HeaderName::from_bytes(name.as_bytes()).map_err(D::Error::custom)?;
            let value = HeaderValue::from_bytes(value.as_bytes()).map_err(D::Error::custom)?;
            headers.append(name, value);
        }
        Ok(headers)
    }
}

pub mod body {
    use super::*;

    pub fn serialize<S: Serializer>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&BASE64.encode(value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        BASE64.decode(encoded.as_bytes()).map_err(D::Error::custom)
    }
}
