//! Serde codecs for the parts of a [`crate::BrowserResponse`] that have no representation of their
//! own: `http`'s `StatusCode` and `HeaderMap` are foreign types, and a captured body is bytes.
//!
//! The wire these feed is what crosses a deployment boundary, so every codec fails closed: an
//! impossible status, a malformed header name or value, a header value that is not text, or a body
//! that is not base64 is an error the caller sees - never a value silently dropped, reordered or
//! replaced with a substitute.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::StatusCode;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

/// `StatusCode` as its `u16`.
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

/// `HeaderMap` as ordered name/value pairs, so a repeated name keeps every value it carried.
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

/// A captured body as base64: the encoding the CDP capture already carries a body in, and the only
/// one that survives a JSON wire without expanding a binary body into an array of integers.
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
