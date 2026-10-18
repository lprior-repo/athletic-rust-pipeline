use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

#[path = "format.rs"]
mod format;
#[path = "build_inputs.rs"]
mod inputs;
#[path = "build_kernel.rs"]
mod kernel;
#[path = "build_process.rs"]
mod process;
#[path = "build_scan.rs"]
mod scan;

pub(super) type BuildResult<T> = Result<T, Box<dyn Error>>;

pub fn generate() -> BuildResult<()> {
    let crate_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let root = crate_root
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root absent")?;
    let manifest = assemble(root)?;
    let bytes = serde_json::to_vec(&manifest)?;
    if bytes.is_empty() || bytes.len() > format::MAX_MANIFEST {
        return Err("roster parser manifest encoded size exceeded".into());
    }
    let destination = PathBuf::from(env::var("OUT_DIR")?);
    kernel::generate(
        root,
        &destination,
        &manifest.configuration,
        &manifest.dependencies,
    )?;
    let manifest_sha256 = inputs::digest("manifest", &bytes)?.sha256;
    std::fs::write(destination.join("roster-parser-manifest-v1.json"), bytes)?;
    let vcs = process::vcs(root, manifest_sha256)?;
    std::fs::write(
        destination.join("roster-parser-vcs-v1.json"),
        serde_json::to_vec(&vcs)?,
    )?;
    Ok(())
}

fn assemble(root: &Path) -> BuildResult<format::Manifest> {
    let mut budget = scan::SourceBudget::default();
    let source_trees = format::SOURCE_ROOTS
        .iter()
        .map(|path| scan::tree(root, path, &mut budget))
        .collect::<BuildResult<Vec<_>>>()?;
    let (total, files) = source_trees
        .iter()
        .try_fold((0_u64, 0_u64), |(bytes, files), tree| {
            Ok::<_, Box<dyn Error>>((
                bytes
                    .checked_add(tree.bytes)
                    .ok_or("roster parser source byte overflow")?,
                files
                    .checked_add(u64::from(tree.files))
                    .ok_or("roster parser source count overflow")?,
            ))
        })?;
    if total > format::MAX_SOURCE_BYTES || files > u64::try_from(format::MAX_FILES)? {
        return Err("roster parser source closure byte limit".into());
    }
    let files = format::CONFIG_FILES
        .iter()
        .map(|path| inputs::file(root, path))
        .collect::<BuildResult<Vec<_>>>()?;
    inputs::watch(root.join(".cargo"))?;
    Ok(format::Manifest {
        version: format::VERSION,
        semantic_policy: format::POLICY.into(),
        closure_policy: format::CLOSURE.into(),
        provenance_scope: "compiled-source-config-lock-inputs-not-binary-or-resolved-features/v1"
            .into(),
        schema: format::Schema {
            contract: "compiled-rust-types-with-entire-source-closure/v1".into(),
            source_trees: source_trees.clone(),
        },
        source_trees,
        configuration: format::Configuration {
            files,
            compiler: process::compiler()?,
            environment: inputs::environment()?,
        },
        dependencies: inputs::file(root, "Cargo.lock")?,
    })
}
