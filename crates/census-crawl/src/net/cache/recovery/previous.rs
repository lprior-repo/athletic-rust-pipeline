use super::{admission::Admission, codec, paths, quarantine, shell, stage, stream};
use crate::net::cache::quarantine::Damage;
use crate::net::cache::CacheMeta;
use crate::net::{CaptureTransportError, FetchError};
use census_store::RecoveryArchiveIo;
use std::path::{Path, PathBuf};

pub(super) enum Previous {
    Absent,
    Damaged {
        presence: (bool, bool),
        damage: Damage,
    },
    Valid {
        metadata: Box<CacheMeta>,
        canonical: Vec<u8>,
    },
}

pub(super) struct Pending {
    pub(super) reservation: Option<PathBuf>,
    paths: Option<OldPaths>,
}
struct OldPaths {
    body: PathBuf,
    metadata: PathBuf,
    missing: (bool, bool),
}

pub(super) fn read(
    body: &Path,
    metadata: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Previous, FetchError> {
    let presence = (
        shell::regular(body, io)?.is_some(),
        shell::regular(metadata, io)?.is_some(),
    );
    match presence {
        (false, false) => return Ok(Previous::Absent),
        (true, true) => {}
        _ => {
            return Ok(Previous::Damaged {
                presence,
                damage: Damage::IncompletePair,
            })
        }
    }
    let encoded = match stream::read_metadata(metadata, io) {
        Ok(encoded) => encoded,
        Err(FetchError::Cache { source, .. })
            if source.kind() == std::io::ErrorKind::InvalidData =>
        {
            return Ok(Previous::Damaged {
                presence,
                damage: Damage::MetadataLimitOrChanged,
            });
        }
        Err(error) => return Err(error),
    };
    let decoded = match codec::decode_previous(&encoded, io) {
        Ok(decoded) => decoded,
        Err(FetchError::Decode { .. }) => {
            return Ok(Previous::Damaged {
                presence,
                damage: Damage::InvalidMetadata,
            })
        }
        Err(error) => return Err(error),
    };
    let (meta, canonical) = decoded;
    if !super::super::is_valid_hex64(&meta.content_digest)
        || meta.bytes > crate::net::MAX_BODY_BYTES
    {
        return Ok(Previous::Damaged {
            presence,
            damage: Damage::InvalidMetadata,
        });
    }
    io.release_owned(encoded.capacity())?;
    drop(encoded);
    io.reserve_owned(std::mem::size_of::<CacheMeta>())?;
    Ok(Previous::Valid {
        metadata: Box::new(meta),
        canonical,
    })
}

pub(super) fn reserve(
    previous: &Previous,
    root: &Path,
    admission: &Admission<'_>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Pending, FetchError> {
    let Previous::Valid {
        metadata,
        canonical,
    } = previous
    else {
        return Ok(Pending {
            reservation: None,
            paths: None,
        });
    };
    let digest_bytes = stream::digest(canonical)?;
    let digest = std::str::from_utf8(&digest_bytes).map_err(|_| shell::limit("digest encoding"))?;
    let archive = paths::child(root, "archive", io)?;
    let bodies = paths::child(&archive, "bodies", io)?;
    let captures = paths::child(&archive, "captures", io)?;
    let content = paths::child(&captures, &metadata.content_digest, io)?;
    paths::safe_directory(&content, io)?;
    let body = paths::suffixed(&bodies, &metadata.content_digest, ".body", io)?;
    let meta = paths::suffixed(&content, digest, ".meta.json", io)?;
    let missing = (
        shell::regular(&body, io)?.is_none(),
        shell::regular(&meta, io)?.is_none(),
    );
    let body_size = if missing.0 {
        u64::try_from(metadata.bytes).map_err(|_| shell::limit("body length"))?
    } else {
        0
    };
    let meta_size = if missing.1 {
        u64::try_from(canonical.len()).map_err(|_| shell::limit("metadata length"))?
    } else {
        0
    };
    let reservation =
        admission.reserve(&metadata.content_digest, digest, (body_size, meta_size), io)?;
    for path in [&archive, &bodies, &captures, &content] {
        io.release_owned(path.as_os_str().len())?;
    }
    Ok(Pending {
        reservation: Some(reservation),
        paths: Some(OldPaths {
            body,
            metadata: meta,
            missing,
        }),
    })
}

pub(super) fn preserve(
    previous: Previous,
    pending: &Pending,
    stage_path: &Path,
    body: &Path,
    metadata: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    match previous {
        Previous::Absent => Ok(()),
        Previous::Damaged { presence, damage } => {
            quarantine::pair(stage_path, body, metadata, presence, damage, io)
        }
        Previous::Valid {
            metadata: meta,
            canonical,
        } => {
            let meta = *meta;
            io.release_owned(std::mem::size_of::<CacheMeta>())?;
            let paths = pending
                .paths
                .as_ref()
                .ok_or(CaptureTransportError::BackingUnavailable)?;
            let bytes = u64::try_from(meta.bytes).map_err(|_| shell::limit("body length"))?;
            let source = preserve_body(
                stage_path,
                body,
                &paths.body,
                paths.missing.0,
                bytes,
                &meta.content_digest,
                io,
            );
            if let Err(failure) = source {
                match failure {
                    BodyFailure::Source(error) if is_damage(&error) => {
                        return quarantine::pair(
                            stage_path,
                            body,
                            metadata,
                            (true, true),
                            Damage::BodyIntegrity,
                            io,
                        );
                    }
                    BodyFailure::Source(error) | BodyFailure::Archive(error) => return Err(error),
                }
            }
            preserve_metadata(paths, stage_path, &canonical, io)
        }
    }
}

fn preserve_metadata(
    paths: &OldPaths,
    stage_path: &Path,
    canonical: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    if paths.missing.1 {
        let leaf = stage::write_leaf(stage_path, "old.meta.json", canonical, io)?;
        stage::publish(&leaf, &paths.metadata, io)?;
        io.release_owned(leaf.as_os_str().len())?;
    }
    stream::metadata_exact(&paths.metadata, canonical, io)?;
    shell::freeze(&paths.metadata, io)?;
    shell::sync_directory(
        paths
            .metadata
            .parent()
            .ok_or_else(|| shell::limit("archive ancestry"))?,
        io,
    )
}

enum BodyFailure {
    Source(FetchError),
    Archive(FetchError),
}

fn preserve_body(
    stage_path: &Path,
    source: &Path,
    destination: &Path,
    missing: bool,
    bytes: u64,
    digest: &str,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), BodyFailure> {
    if missing {
        let staged = paths::child(stage_path, "old.body", io).map_err(BodyFailure::Archive)?;
        let mut copy = shell::create(&staged, io).map_err(BodyFailure::Archive)?;
        stream::body(source, bytes, digest, Some((&mut copy, &staged)), io)
            .map_err(BodyFailure::Source)?;
        shell::sync(&copy, &staged, io).map_err(BodyFailure::Archive)?;
        shell::freeze(&staged, io).map_err(BodyFailure::Archive)?;
        stage::publish(&staged, destination, io).map_err(BodyFailure::Archive)?;
        io.release_owned(staged.as_os_str().len())
            .map_err(|error| BodyFailure::Archive(error.into()))?;
    } else {
        stream::body(source, bytes, digest, None, io).map_err(BodyFailure::Source)?;
    }
    stream::body(destination, bytes, digest, None, io).map_err(BodyFailure::Archive)?;
    shell::freeze(destination, io).map_err(BodyFailure::Archive)?;
    shell::sync_directory(
        destination
            .parent()
            .ok_or_else(|| BodyFailure::Archive(shell::limit("archive ancestry")))?,
        io,
    )
    .map_err(BodyFailure::Archive)
}

fn is_damage(error: &FetchError) -> bool {
    matches!(error, FetchError::Cache { source, .. } if source.kind() == std::io::ErrorKind::InvalidData)
}
