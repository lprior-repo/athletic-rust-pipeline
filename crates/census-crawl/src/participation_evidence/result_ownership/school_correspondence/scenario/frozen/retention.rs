use crate::milesplit::SourceRowLocator;
use census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1;
use census_domain::model::SchoolId;
use census_domain::model::StoreIdentity;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const PROOF_MAX_BYTES: u64 = 4 * 1024 * 1024;
const MANIFEST_MAX_BYTES: u64 = 65536;

fn retention_directory() -> io::Result<Option<PathBuf>> {
    match env::var_os("CENSUS_RESULT_SCHOOL_RETENTION_DIR") {
        None => Ok(None),
        Some(raw) => {
            let candidate = PathBuf::from(raw);
            if !candidate.is_absolute() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "retention directory is not absolute",
                ));
            }
            let metadata = candidate.symlink_metadata()?;
            if metadata.is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "retention directory is a symlink",
                ));
            }
            if !metadata.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "retention directory does not exist or is not a directory",
                ));
            }
            Ok(Some(candidate))
        }
    }
}

fn stage_name(expected: Option<&SchoolId>) -> &'static str {
    if expected.is_some() {
        "accepted"
    } else {
        "revoked"
    }
}

fn check_existing_subdir(subdir: &Path) -> io::Result<bool> {
    match subdir.symlink_metadata() {
        Ok(meta) => {
            if meta.is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "retention target is a symlink",
                ));
            }
            if !meta.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "retention target is not a directory",
                ));
            }
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn check_file(path: &Path) -> io::Result<bool> {
    let meta = path.symlink_metadata()?;
    if meta.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "existing file is a symlink",
        ));
    }
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "existing file is not a regular file",
        ));
    }
    Ok(true)
}

fn bounded_read_file(path: &Path, cap: u64) -> io::Result<Vec<u8>> {
    let meta = path.symlink_metadata()?;
    let size = meta.len();
    if size > cap {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file exceeds maximum size",
        ));
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let expected = usize::try_from(size)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file length overflow"))?;
    let mut file = file;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(expected)
        .map_err(io::Error::other)?;
    buffer.resize(expected, 0);
    file.read_exact(&mut buffer)?;
    let mut extra = [0u8; 1];
    if file.read(&mut extra)? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file grew during read",
        ));
    }
    Ok(buffer)
}

fn compare_existing_files(
    subdir: &Path,
    stage: &str,
    proof_bytes: &[u8],
    manifest_bytes: &[u8],
) -> io::Result<bool> {
    let proof_path = subdir.join(format!("{}.proof.json", stage));
    let manifest_path = subdir.join(format!("{}.manifest.json", stage));

    check_file(&proof_path)?;
    check_file(&manifest_path)?;

    let existing_proof = bounded_read_file(&proof_path, PROOF_MAX_BYTES)?;
    if existing_proof != proof_bytes {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "existing proof differs",
        ));
    }

    let existing_manifest = bounded_read_file(&manifest_path, MANIFEST_MAX_BYTES)?;
    if existing_manifest != manifest_bytes {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "existing manifest differs",
        ));
    }

    Ok(true)
}

fn is_idempotent(subdir: &Path, stage: &str, proof: &[u8], manifest: &[u8]) -> io::Result<bool> {
    if !check_existing_subdir(subdir)? {
        return Ok(false);
    }
    compare_existing_files(subdir, stage, proof, manifest)
}

#[derive(serde::Serialize)]
struct Manifest<'a> {
    version: u64,
    stage: &'a str,
    store: &'a StoreIdentity,
    snapshot_sequence: u64,
    proof_ref: &'a AttachmentObjectReferenceV1,
    proof_file: &'a str,
    expected_school: Option<&'a SchoolId>,
    result_context: &'a str,
    result_locator: &'a SourceRowLocator,
}

fn write_proof_file(subdir: &Path, stage: &str, proof: &[u8]) -> io::Result<String> {
    let file_name = format!("{}.proof.json", stage);
    let path = subdir.join(&file_name);
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(proof)?;
        file.sync_all()?;
    }
    Ok(file_name)
}

fn write_manifest_file(subdir: &Path, stage: &str, manifest: &[u8]) -> io::Result<()> {
    let path = subdir.join(format!("{}.manifest.json", stage));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(manifest)?;
    file.sync_all()?;
    Ok(())
}

fn write_retention(
    retention_dir: &Path,
    proof_bytes: &[u8],
    proof_reference: &AttachmentObjectReferenceV1,
    store: &StoreIdentity,
    snapshot_sequence: u64,
    input_context: &str,
    expected: Option<&SchoolId>,
    locator: &SourceRowLocator,
) -> io::Result<()> {
    let proof_len: u64 = proof_bytes
        .len()
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "proof length overflow"))?;
    if proof_len > PROOF_MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "proof exceeds 4MiB limit",
        ));
    }

    let stage = stage_name(expected);
    let proof_file_name = format!("{}.proof.json", stage);
    let unique = format!("{}-{}", stage, snapshot_sequence);
    let subdir = retention_dir.join(unique);

    let manifest = Manifest {
        version: 1,
        stage,
        store,
        snapshot_sequence,
        proof_ref: proof_reference,
        proof_file: &proof_file_name,
        expected_school: expected,
        result_context: input_context,
        result_locator: locator,
    };
    let manifest_bytes = serde_json::to_vec(&manifest).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("manifest serialization: {e}"),
        )
    })?;

    let manifest_len: u64 = manifest_bytes
        .len()
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "manifest length overflow"))?;
    if manifest_len > MANIFEST_MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "manifest exceeds 64KiB limit",
        ));
    }

    if is_idempotent(&subdir, stage, proof_bytes, &manifest_bytes)? {
        return Ok(());
    }

    fs::create_dir(&subdir)?;
    write_proof_file(&subdir, stage, proof_bytes)?;
    write_manifest_file(&subdir, stage, &manifest_bytes)?;
    Ok(())
}

pub fn attempt_retention(
    proof_bytes: &[u8],
    proof_reference: &AttachmentObjectReferenceV1,
    store: &StoreIdentity,
    snapshot_sequence: u64,
    input_context: &str,
    expected: Option<&SchoolId>,
    locator: &SourceRowLocator,
) -> io::Result<()> {
    let retention_dir = match retention_directory()? {
        Some(dir) => dir,
        None => return Ok(()),
    };
    write_retention(
        &retention_dir,
        proof_bytes,
        proof_reference,
        store,
        snapshot_sequence,
        input_context,
        expected,
        locator,
    )
}
