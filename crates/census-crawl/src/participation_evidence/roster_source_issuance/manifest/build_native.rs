use super::{
    format, inputs, write_document, BuildResult, Kernel, MAX_DOCUMENT, SCHEMA_V12, SCHEMA_V13,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

const TEMPLATE_PATH: &str = "crates/census-crawl/src/participation_evidence/roster_source_issuance/historical/kernel_adapter/fixtures/kernel-v13-context-native.json";
const NATIVE_CONTEXT: &str = "a01453e2391425f2cb50260028d1f5d75ad23e5adef7fba807ec95db38d3787d";

#[derive(Deserialize)]
struct Template {
    configuration: format::Configuration,
    dependencies: format::Input,
}

pub(super) fn generate(
    root: &Path,
    destination: &Path,
    source_digest: &str,
    sources: &[format::Input],
) -> BuildResult<()> {
    let template = load_template(root)?;
    let mut kernel = Kernel {
        schema: SCHEMA_V12,
        source_digest,
        sources,
        configuration: &template.configuration,
        dependencies: &template.dependencies,
    };
    let mut bytes = serde_json::to_vec(&kernel)?;
    write_document(
        destination,
        "roster-semantic-kernel-v12-native.json",
        &bytes,
    )?;
    write_digest(
        destination,
        "roster-semantic-kernel-v12-native.sha256",
        &bytes,
    )?;
    kernel.schema = SCHEMA_V13;
    bytes.clear();
    serde_json::to_writer(&mut bytes, &kernel)?;
    write_document(
        destination,
        "roster-semantic-kernel-v13-native.json",
        &bytes,
    )?;
    write_digest(
        destination,
        "roster-semantic-kernel-v13-native.sha256",
        &bytes,
    )?;
    Ok(())
}

fn load_template(root: &Path) -> BuildResult<Template> {
    let input = inputs::file_limited(root, TEMPLATE_PATH, u64::try_from(MAX_DOCUMENT)?)?;
    let bytes = std::fs::read(root.join(TEMPLATE_PATH))?;
    let actual = inputs::digest(TEMPLATE_PATH, &bytes)?;
    if actual != input {
        return Err("native kernel context template changed during read".into());
    }
    let template: Template = serde_json::from_slice(&bytes)?;
    let configuration = serde_json::to_vec(&template.configuration)?;
    let dependencies = serde_json::to_vec(&template.dependencies)?;
    let mut digest = Sha256::new();
    digest.update(u64::try_from(configuration.len())?.to_le_bytes());
    digest.update(configuration);
    digest.update(dependencies);
    if format!("{:x}", digest.finalize()) != NATIVE_CONTEXT {
        return Err("unsupported native kernel context template".into());
    }
    Ok(template)
}

fn write_digest(destination: &Path, name: &str, bytes: &[u8]) -> BuildResult<()> {
    std::fs::write(destination.join(name), inputs::digest(name, bytes)?.sha256)?;
    Ok(())
}
