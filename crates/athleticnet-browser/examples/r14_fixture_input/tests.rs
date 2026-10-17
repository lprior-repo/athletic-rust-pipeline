#[path = "../r14_native_fixture/gzip.rs"]
mod gzip;

use super::{
    read_artifact, read_manifest, ArtifactKind, ArtifactManifest, FixtureInputError,
    FixtureManifest,
};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn with_directory(test: impl FnOnce(&Path) -> TestResult) -> TestResult {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let directory = std::env::temp_dir().join(format!(
        "r14-reader-{}-{}",
        std::process::id(),
        stamp.as_nanos()
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
    let result = test(&directory);
    let cleanup = std::fs::remove_dir_all(&directory);
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Err(error), Err(cleanup)) => {
            Err(format!("{error}; fixture cleanup failed: {cleanup}").into())
        }
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> TestResult {
    let mut file = gzip::create_private(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn artifact(summary: gzip::Summary) -> ArtifactManifest {
    ArtifactManifest {
        encoded: summary.encoded,
        decoded: summary.decoded,
        sha256: summary.sha256,
        decoded_sha256: summary.decoded_sha256,
    }
}

fn rehash(manifest: &mut ArtifactManifest, bytes: &[u8]) -> TestResult {
    manifest.encoded = u64::try_from(bytes.len())?;
    manifest.sha256 = format!("{:x}", Sha256::digest(bytes));
    Ok(())
}

#[test]
fn manifest_accepts_exact_byte_limit_and_refuses_the_next_byte() -> TestResult {
    with_directory(|directory| {
        let zero_digest = "0".repeat(64);
        let manifest = FixtureManifest {
            fixture_dir: directory
                .to_str()
                .ok_or("fixture path is not UTF-8")?
                .to_owned(),
            normal_body: gzip::NORMAL_BODY.to_owned(),
            wire: ArtifactManifest {
                encoded: gzip::WIRE_ENCODED,
                decoded: gzip::WIRE_DECODED,
                sha256: zero_digest.clone(),
                decoded_sha256: zero_digest.clone(),
            },
            decoded: ArtifactManifest {
                encoded: 1,
                decoded: gzip::DECODED_DECODED,
                sha256: zero_digest.clone(),
                decoded_sha256: zero_digest,
            },
            scope: gzip::SCOPE.to_owned(),
        };
        let mut bytes = serde_json::to_vec(&manifest)?;
        let reserve = 8193usize
            .checked_sub(bytes.len())
            .ok_or("manifest fixture exceeds bound")?;
        bytes.try_reserve_exact(reserve)?;
        bytes.resize(8192, b' ');
        let exact = directory.join("exact.json");
        write_file(&exact, &bytes)?;
        let accepted = read_manifest(&exact)?;
        if accepted.scope != manifest.scope {
            return Err("the exact-boundary manifest was not decoded".into());
        }
        bytes.push(b' ');
        let over = directory.join("over.json");
        write_file(&over, &bytes)?;
        match read_manifest(&over) {
            Err(FixtureInputError::Io(error)) if error.kind() == std::io::ErrorKind::Other => {
                Ok(())
            }
            outcome => Err(format!(
                "the over-bound manifest was not refused before parsing: {outcome:?}"
            )
            .into()),
        }
    })
}

#[test]
fn artifact_rejects_crc_corruption_after_the_outer_digest_is_updated() -> TestResult {
    with_directory(|directory| {
        let original = directory.join("original.gz");
        let mut manifest = artifact(gzip::write_decoded(&original)?);
        let mut bytes = read_artifact(&original, &manifest, ArtifactKind::Decoded)?;
        let crc_offset = bytes
            .len()
            .checked_sub(8)
            .ok_or("gzip fixture has no footer")?;
        let crc = bytes
            .get_mut(crc_offset)
            .ok_or("gzip CRC offset is absent")?;
        *crc ^= 1;
        rehash(&mut manifest, &bytes)?;
        let corrupt = directory.join("corrupt-crc.gz");
        write_file(&corrupt, &bytes)?;
        match read_artifact(&corrupt, &manifest, ArtifactKind::Decoded) {
            Err(FixtureInputError::Io(_)) => Ok(()),
            outcome => Err(format!("self-consistently rehashed CRC corruption was not rejected by decoding: {outcome:?}").into()),
        }
    })
}

#[test]
fn artifact_requires_complete_gzip_and_exact_end_of_input_after_rehashing() -> TestResult {
    with_directory(|directory| {
        let original = directory.join("original.gz");
        let mut manifest = artifact(gzip::write_decoded(&original)?);
        let mut bytes = read_artifact(&original, &manifest, ArtifactKind::Decoded)?;
        bytes.try_reserve(1)?;
        let last = bytes.pop().ok_or("gzip fixture is empty")?;
        rehash(&mut manifest, &bytes)?;
        let truncated = directory.join("truncated.gz");
        write_file(&truncated, &bytes)?;
        match read_artifact(&truncated, &manifest, ArtifactKind::Decoded) {
            Err(FixtureInputError::Io(_)) => {}
            outcome => {
                return Err(
                    format!("truncated gzip was not refused by decoding: {outcome:?}").into(),
                )
            }
        }
        bytes.push(last);
        bytes.push(0);
        rehash(&mut manifest, &bytes)?;
        let trailing = directory.join("trailing.gz");
        write_file(&trailing, &bytes)?;
        match read_artifact(&trailing, &manifest, ArtifactKind::Decoded) {
            Err(FixtureInputError::Invalid(_)) => Ok(()),
            outcome => Err(format!(
                "trailing bytes were accepted despite an exact-EOF contract: {outcome:?}"
            )
            .into()),
        }
    })
}

#[test]
fn artifact_with_consistent_sizes_and_hashes_still_requires_the_declared_content() -> TestResult {
    with_directory(|directory| {
        let path = directory.join("wrong-content.gz");
        let file = gzip::create_private(&path)?;
        let mut encoder = flate2::GzBuilder::new()
            .mtime(0)
            .write(file, flate2::Compression::best());
        let chunk = [1u8; 65_536];
        let mut remaining = gzip::DECODED_DECODED;
        let mut decoded_hash = Sha256::new();
        while remaining != 0 {
            let count = usize::try_from(remaining.min(u64::try_from(chunk.len())?))?;
            let bytes = chunk
                .get(..count)
                .ok_or("decoded fixture chunk is absent")?;
            encoder.write_all(bytes)?;
            decoded_hash.update(bytes);
            remaining = remaining
                .checked_sub(u64::try_from(count)?)
                .ok_or("decoded count underflow")?;
        }
        encoder.finish()?.sync_all()?;
        let bytes = std::fs::read(&path)?;
        let manifest = ArtifactManifest {
            encoded: u64::try_from(bytes.len())?,
            decoded: gzip::DECODED_DECODED,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            decoded_sha256: format!("{:x}", decoded_hash.finalize()),
        };
        match read_artifact(&path, &manifest, ArtifactKind::Decoded) {
            Err(FixtureInputError::Invalid(_)) => Ok(()),
            outcome => Err(format!(
                "self-consistent wrong-content fixture was accepted: {outcome:?}"
            )
            .into()),
        }
    })
}
