use crate::{AdapterContext, CrawlError, CrawlResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

const PHASE: &str = "meet_source_plans_v1";

#[derive(Serialize, Deserialize)]
struct Frozen<T> {
    digest: String,
    encoded_bytes: usize,
    payload: T,
}

pub(crate) fn read<T: DeserializeOwned + Serialize>(
    ctx: &AdapterContext<'_>,
    label: &str,
) -> CrawlResult<Option<T>> {
    let key = key(ctx, label)?;
    let Some(value) = ctx.store.journal_payload(PHASE, &key)? else {
        return Ok(None);
    };
    let frozen: Frozen<T> = serde_json::from_value(value).map_err(|source| CrawlError::Decode {
        url: key.clone(),
        source,
    })?;
    let (digest, bytes) = fingerprint(&frozen.payload)?;
    if digest != frozen.digest || bytes != frozen.encoded_bytes {
        return Err(CrawlError::Invariant {
            detail: format!("frozen meet plan changed: {key}"),
        });
    }
    Ok(Some(frozen.payload))
}

pub(crate) fn write<T: Serialize>(
    ctx: &AdapterContext<'_>,
    label: &str,
    payload: &T,
) -> CrawlResult<()> {
    let (digest, encoded_bytes) = fingerprint(payload)?;
    ctx.store.journal_done(
        PHASE,
        &key(ctx, label)?,
        &Frozen {
            digest,
            encoded_bytes,
            payload,
        },
    )?;
    Ok(())
}

fn key(ctx: &AdapterContext<'_>, label: &str) -> CrawlResult<String> {
    let identity = ctx
        .recording
        .and_then(super::Recording::fragment_identity)
        .and_then(|identity| identity.rsplit_once('/').map(|(base, _)| base))
        .ok_or_else(|| CrawlError::Invariant {
            detail: "meet fragment requires a durable source identity".into(),
        })?;
    Ok(format!("{identity}/{label}"))
}

fn fingerprint<T: Serialize>(payload: &T) -> CrawlResult<(String, usize)> {
    let mut writer = Fingerprint {
        hash: Sha256::new(),
        bytes: 0,
        refused: false,
    };
    let encoded = serde_json::to_writer(&mut writer, payload);
    if writer.refused {
        return Err(super::budget::resource(
            "frozen meet plan bytes",
            writer.bytes,
            super::MAX_RECORDED_BYTES,
        ));
    }
    encoded.map_err(|source| CrawlError::Decode {
        url: "frozen meet plan".into(),
        source,
    })?;
    Ok((format!("{:x}", writer.hash.finalize()), writer.bytes))
}

struct Fingerprint {
    hash: Sha256,
    bytes: usize,
    refused: bool,
}

impl Write for Fingerprint {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("meet plan byte overflow"))?;
        if self.bytes > super::MAX_RECORDED_BYTES {
            self.refused = true;
            return Err(io::Error::other("meet plan byte ceiling"));
        }
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
