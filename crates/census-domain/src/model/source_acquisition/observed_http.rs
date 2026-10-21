use super::{AcquisitionDigest, CaptureDomainError, MAX_ACQUISITION_BODY_BYTES};
use serde::{Deserialize, Serialize};

mod chain;
mod codec;
mod header;
mod messages;
mod validation;

pub(super) use header::valid_header_value;
pub use header::ObservedHttpHeaderV1;
pub use messages::{ObservedHttpHopV1, ObservedHttpRequestV1, ObservedHttpResponseV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedHttpProtocolV1 {
    Http10,
    Http11,
    Http2,
    Http3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Recipe {
    ReqwestNormalizedObservedHttpV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BodyEncoding {
    Identity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ObservedWire")]
pub struct NormalizedObservedHttpV1 {
    recipe: Recipe,
    hops: Vec<ObservedHttpHopV1>,
    completed_at: String,
    body_digest: AcquisitionDigest,
    body_bytes: u64,
    body_encoding: BodyEncoding,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedWire {
    recipe: Recipe,
    #[serde(deserialize_with = "chain::deserialize_hops")]
    hops: Vec<ObservedHttpHopV1>,
    completed_at: String,
    body_digest: AcquisitionDigest,
    body_bytes: u64,
    body_encoding: BodyEncoding,
}

impl NormalizedObservedHttpV1 {
    pub fn new(
        hops: Vec<ObservedHttpHopV1>,
        completed_at: &str,
        body_digest: AcquisitionDigest,
        body_bytes: u64,
    ) -> Result<Self, CaptureDomainError> {
        if body_bytes > MAX_ACQUISITION_BODY_BYTES {
            return Err(CaptureDomainError::Bound {
                field: "observed body bytes",
                max: 32 * 1024 * 1024,
            });
        }
        chain::validate(&hops, completed_at)?;
        let observed = Self {
            recipe: Recipe::ReqwestNormalizedObservedHttpV1,
            hops,
            completed_at: completed_at.to_owned(),
            body_digest,
            body_bytes,
            body_encoding: BodyEncoding::Identity,
        };
        codec::ensure_limit(&observed)?;
        Ok(observed)
    }

    pub fn hops(&self) -> &[ObservedHttpHopV1] {
        &self.hops
    }

    pub fn completed_at(&self) -> &str {
        &self.completed_at
    }

    pub fn body_digest(&self) -> &AcquisitionDigest {
        &self.body_digest
    }

    pub fn body_bytes(&self) -> u64 {
        self.body_bytes
    }
}

impl TryFrom<ObservedWire> for NormalizedObservedHttpV1 {
    type Error = CaptureDomainError;

    fn try_from(wire: ObservedWire) -> Result<Self, Self::Error> {
        let (Recipe::ReqwestNormalizedObservedHttpV1, BodyEncoding::Identity) =
            (wire.recipe, wire.body_encoding);
        Self::new(
            wire.hops,
            &wire.completed_at,
            wire.body_digest,
            wire.body_bytes,
        )
    }
}

#[cfg(test)]
mod tests;
