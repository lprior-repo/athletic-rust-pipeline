#![forbid(unsafe_code)]

#[path = "r14_fixture_input/schema.rs"]
mod fixture_schema;

#[path = "r14_native_fixture/gzip.rs"]
mod gzip;

#[path = "r14_native_fixture/wire.rs"]
mod wire;

use std::fs::{self, DirBuilder};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use gzip::create_private;

use gzip::{Summary, NORMAL_BODY, SCOPE};

#[derive(thiserror::Error)]
enum FixtureError {
    #[error("usage: r14_native_fixture <freshRoot>")]
    Usage,
    #[error("fixture root already exists: {0:?}")]
    RootExists(PathBuf),
    #[error("fixture metadata {path:?}: {source}")]
    Metadata {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("fixture I/O {path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("fixture root mode {mode:o}, expected 0700")]
    RootMode { mode: u32 },
    #[error("fixture root is not a directory: {0:?}")]
    RootType(PathBuf),
    #[error("fixture path is not UTF-8: {0:?}")]
    NonUtf8(PathBuf),
    #[error("fixture manifest: {0}")]
    Manifest(serde_json::Error),
}

impl std::fmt::Debug for FixtureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

fn main() -> Result<(), FixtureError> {
    let mut arguments = std::env::args_os();
    if arguments.next().is_none() {
        return Err(FixtureError::Usage);
    }
    let Some(raw) = arguments.next() else {
        return Err(FixtureError::Usage);
    };
    if arguments.next().is_some() {
        return Err(FixtureError::Usage);
    }
    let root = PathBuf::from(raw);
    let fixture_dir = root
        .to_str()
        .ok_or_else(|| FixtureError::NonUtf8(root.clone()))?
        .to_owned();
    require_absent(&root)?;
    create_root(&root)?;
    let wire_path = root.join("wire-over.gz");
    let wire = wire::write_wire(&wire_path).map_err(|source| fault(&wire_path, source))?;
    let decoded_path = root.join("decoded-over.gz");
    let decoded =
        gzip::write_decoded(&decoded_path).map_err(|source| fault(&decoded_path, source))?;
    write_manifest(&root, fixture_dir, wire, decoded)?;
    Ok(())
}

fn require_absent(root: &Path) -> Result<(), FixtureError> {
    match fs::symlink_metadata(root) {
        Ok(_) => Err(FixtureError::RootExists(root.to_path_buf())),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FixtureError::Metadata {
            path: root.to_path_buf(),
            source,
        }),
    }
}

fn create_root(root: &Path) -> Result<(), FixtureError> {
    DirBuilder::new()
        .mode(0o700)
        .create(root)
        .map_err(|source| fault(root, source))?;
    let metadata = fs::symlink_metadata(root).map_err(|source| FixtureError::Metadata {
        path: root.to_path_buf(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(FixtureError::RootType(root.to_path_buf()));
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o700 {
        return Err(FixtureError::RootMode { mode });
    }
    Ok(())
}

fn fault(path: &Path, source: std::io::Error) -> FixtureError {
    FixtureError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn write_manifest(
    root: &Path,
    fixture_dir: String,
    wire: Summary,
    decoded: Summary,
) -> Result<(), FixtureError> {
    let manifest = fixture_schema::FixtureManifest {
        fixture_dir,
        normal_body: NORMAL_BODY.to_owned(),
        wire: fixture_schema::ArtifactManifest {
            encoded: wire.encoded,
            decoded: wire.decoded,
            sha256: wire.sha256,
            decoded_sha256: wire.decoded_sha256,
        },
        decoded: fixture_schema::ArtifactManifest {
            encoded: decoded.encoded,
            decoded: decoded.decoded,
            sha256: decoded.sha256,
            decoded_sha256: decoded.decoded_sha256,
        },
        scope: SCOPE.to_owned(),
    };
    fixture_schema::validate_manifest(&manifest)
        .map_err(|source| fault(root, std::io::Error::other(source)))?;
    let path = root.join("manifest.json");
    let mut file = create_private(&path).map_err(|source| fault(&path, source))?;
    serde_json::to_writer_pretty(&mut file, &manifest).map_err(FixtureError::Manifest)?;
    file.flush().map_err(|source| fault(&path, source))?;
    file.sync_all().map_err(|source| fault(&path, source))?;
    Ok(())
}
