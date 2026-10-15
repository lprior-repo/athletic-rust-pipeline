use super::{digest, invalid, objects::Reference};
use crate::export::proof_objects;
use crate::report::ReportResult;
use census_store::{AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2, StoreError};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CHUNK_BYTES: usize = 4 * 1024 * 1024;
const CHUNK_CEILING: u64 = 4 * 1024 * 1024;
const MAX_BYTES: usize = census_store::MAX_FROZEN_PUBLIC_IDENTITY_BYTES;
const MAX_CHUNKS: usize = MAX_BYTES / CHUNK_BYTES;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bundle {
    digest: String,
    bytes: usize,
    #[serde(deserialize_with = "deserialize_chunks")]
    chunks: Vec<Reference>,
}

impl Bundle {
    pub(super) fn capture(
        directory: &Path,
        bundle: &FrozenPublicIdentityBundleV2,
    ) -> ReportResult<Self> {
        let bytes = bundle.bytes();
        if bytes.is_empty() || bytes.len() > MAX_BYTES || !digest(bundle.digest()) {
            return Err(invalid(
                "selected public identity bundle exceeds its shape or byte limit",
            ));
        }
        let mut chunks = Vec::new();
        chunks
            .try_reserve_exact(bytes.len().div_ceil(CHUNK_BYTES))
            .map_err(|_| invalid("selected identity chunk inventory allocation refused"))?;
        for chunk in bytes.chunks(CHUNK_BYTES) {
            chunks.push(Reference {
                digest: proof_objects::install(directory, chunk)?,
                bytes: u64::try_from(chunk.len()).map_err(|_| StoreError::CounterOverflow)?,
            });
        }
        let bundle = Self {
            digest: bundle.digest().into(),
            bytes: bytes.len(),
            chunks,
        };
        bundle.validate()?;
        Ok(bundle)
    }

    pub(super) fn replay(&self, directory: &Path) -> ReportResult<AcceptedPublicIdentitySnapshot> {
        census_store::replay_frozen_public_identity_v2(&self.read(directory)?, &self.digest)
            .map_err(Into::into)
    }

    pub(super) fn replay_historical(
        &self,
        directory: &Path,
    ) -> ReportResult<AcceptedPublicIdentitySnapshot> {
        census_store::replay_frozen_public_identity_v1(&self.read(directory)?, &self.digest)
            .map_err(Into::into)
    }

    fn read(&self, directory: &Path) -> ReportResult<Vec<u8>> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.bytes)
            .map_err(|_| invalid("selected public identity replay allocation refused"))?;
        for chunk in &self.chunks {
            let part = proof_objects::read(directory, &chunk.digest, CHUNK_CEILING)?;
            if u64::try_from(part.len()).map_err(|_| StoreError::CounterOverflow)? != chunk.bytes {
                return Err(invalid(
                    "selected identity chunk differs from its exact byte count",
                ));
            }
            bytes.extend_from_slice(&part);
        }
        Ok(bytes)
    }

    pub(super) fn chunks(&self) -> &[Reference] {
        &self.chunks
    }

    pub(super) fn validate(&self) -> ReportResult<()> {
        if !digest(&self.digest)
            || self.bytes == 0
            || self.bytes > MAX_BYTES
            || self.chunks.len() != self.bytes.div_ceil(CHUNK_BYTES)
            || self.chunks.len() > MAX_CHUNKS
        {
            return Err(invalid(
                "selected identity bundle version-independent bounds differ",
            ));
        }
        let mut remaining = self.bytes;
        for chunk in &self.chunks {
            let expected = remaining.min(CHUNK_BYTES);
            if !digest(&chunk.digest)
                || chunk.bytes
                    != u64::try_from(expected).map_err(|_| StoreError::CounterOverflow)?
            {
                return Err(invalid("selected identity bundle chunk boundary differs"));
            }
            remaining = remaining
                .checked_sub(expected)
                .ok_or(StoreError::CounterOverflow)?;
        }
        if remaining != 0 {
            return Err(invalid(
                "selected identity bundle has an incomplete chunk inventory",
            ));
        }
        Ok(())
    }
}

fn deserialize_chunks<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Reference>, D::Error> {
    super::objects::deserialize_references(deserializer, MAX_CHUNKS)
}
