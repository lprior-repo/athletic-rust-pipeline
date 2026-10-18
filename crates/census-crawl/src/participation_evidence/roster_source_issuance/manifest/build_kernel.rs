use super::{format, inputs, BuildResult};
use serde::Serialize;
use std::path::Path;

const SCHEMA_V8: &str = "roster-semantic-kernel-dependencies/v8";
const SCHEMA_V9: &str = "roster-semantic-kernel-dependencies/v9";
const MAX_DOCUMENT: usize = 64 * 1024;

#[path = "kernel_sources.rs"]
mod kernel_sources;

#[derive(Serialize)]
struct Kernel<'a> {
    schema: &'static str,
    source_digest: &'a str,
    sources: &'a [format::Input],
    configuration: &'a format::Configuration,
    dependencies: &'a format::Input,
}

pub(super) fn generate(
    root: &Path,
    destination: &Path,
    configuration: &format::Configuration,
    dependencies: &format::Input,
) -> BuildResult<()> {
    if kernel_sources::SOURCES.len() > 128 {
        return Err("semantic kernel source count exceeded".into());
    }
    let sources = kernel_sources::SOURCES
        .iter()
        .map(|path| inputs::file(root, path))
        .collect::<BuildResult<Vec<_>>>()?;
    let total = sources.iter().try_fold(0_u64, |held, file| {
        held.checked_add(file.bytes)
            .ok_or("semantic kernel source byte overflow")
    })?;
    if total > format::MAX_SOURCE_BYTES {
        return Err("semantic kernel source bytes exceeded".into());
    }
    let source_digest = inputs::digest("kernel-sources", &serde_json::to_vec(&sources)?)?.sha256;

    let mut kernel = Kernel {
        schema: SCHEMA_V8,
        source_digest: &source_digest,
        sources: &sources,
        configuration,
        dependencies,
    };
    let mut bytes = serde_json::to_vec(&kernel)?;
    write_document(destination, "roster-semantic-kernel-v8.json", &bytes)?;
    kernel.schema = SCHEMA_V9;
    bytes.clear();
    serde_json::to_writer(&mut bytes, &kernel)?;
    write_document(destination, "roster-semantic-kernel-v9.json", &bytes)?;

    Ok(())
}

fn write_document(destination: &Path, name: &str, bytes: &[u8]) -> BuildResult<()> {
    if bytes.is_empty() || bytes.len() > MAX_DOCUMENT {
        return Err("semantic kernel document size exceeded".into());
    }
    std::fs::write(destination.join(name), bytes)?;
    Ok(())
}
