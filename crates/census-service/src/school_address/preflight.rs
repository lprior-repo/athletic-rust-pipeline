use super::generation_error::{io, GenerationError};
use super::{manifest, SchoolAddressArgs};
use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn check(args: &SchoolAddressArgs) -> Result<()> {
    let names = [
        "school_directory.json",
        "school_directory.csv",
        "changes.json",
        "pipeline_report.json",
        "baseline.json",
        "update_ledger.json",
        "manifest.json",
    ];
    let mut destinations = BTreeSet::new();
    for name in names {
        destinations.insert(args.out.join(name));
    }
    for path in [&args.baseline, &args.ledger].into_iter().flatten() {
        if !destinations.insert(path.clone()) {
            bail!("two outputs name {}", path.display());
        }
    }
    for name in names {
        let path = args.out.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                return Err(GenerationError::Destination {
                    path,
                    detail: "legacy flat layout is rejected; choose a fresh output directory"
                        .to_string(),
                }
                .into());
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(io(&path, source).into()),
        }
    }
    if args.out.exists() {
        if !args.out.is_dir() {
            return Err(blocked(&args.out, "output is not a directory").into());
        }
        for name in ["generations", "current"] {
            let path = args.out.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if name == "generations" && !meta.file_type().is_dir() => {
                    return Err(blocked(&path, "generations must be a real directory").into());
                }
                Ok(meta) if name == "current" && !meta.file_type().is_symlink() => {
                    return Err(blocked(&path, "current must be a symlink").into());
                }
                Ok(_) if name == "current" => {
                    manifest::verify_current(&args.out)?;
                }
                Ok(_) => {}
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(io(&path, source).into()),
            }
        }
    }
    Ok(())
}

fn blocked(path: &Path, detail: &str) -> GenerationError {
    GenerationError::Destination {
        path: path.to_path_buf(),
        detail: detail.to_string(),
    }
}
