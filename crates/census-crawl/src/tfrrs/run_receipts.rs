use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::CanonicalSchool;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;

pub(super) const PHASE: &str = "tfrrs_projection_v2";

pub(super) fn context(
    ctx: &AdapterContext<'_>,
    schools: &[CanonicalSchool],
) -> CrawlResult<String> {
    digest(&(PHASE, ctx.performance_as_of, ctx.school_year, schools))
}

pub(super) fn key(context: &str, url: &str, body_sha256: &str) -> CrawlResult<String> {
    digest(&(context, url, body_sha256))
}

fn digest(value: &impl Serialize) -> CrawlResult<String> {
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value).map_err(|error| CrawlError::Invariant {
        detail: format!("TFRRS projection receipt serialization: {error}"),
    })?;
    let mut result = String::new();
    result
        .try_reserve_exact(64)
        .map_err(|_| CrawlError::Resource {
            resource: "TFRRS projection receipt bytes",
            requested: 64,
            limit: 64,
        })?;
    writer.0.finalize().iter().try_for_each(|byte| {
        std::fmt::Write::write_fmt(&mut result, format_args!("{byte:02x}")).map_err(|error| {
            CrawlError::Invariant {
                detail: format!("TFRRS projection receipt encoding: {error}"),
            }
        })
    })?;
    Ok(result)
}

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
