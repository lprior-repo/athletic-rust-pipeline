use super::validate;
use super::{ArchiveActivityReason, MAX_ACTIVITY_META_TOTAL_BYTES};
use std::path::Path;

pub(super) struct Tallies {
    visited: u64,
    documents: u64,
    bodies: u64,
    body_bytes: u64,
    meta_bytes: u64,
}

pub(super) fn count_addresses(archive: &Path) -> Result<(u64, u64, u64), ArchiveActivityReason> {
    let mut tallies = Tallies {
        visited: 0,
        documents: 0,
        bodies: 0,
        body_bytes: 0,
        meta_bytes: 0,
    };
    check_top(archive, &mut tallies)?;
    count_bodies(&archive.join("bodies"), &mut tallies)?;
    count_captures(&archive.join("captures"), &mut tallies)?;
    Ok((tallies.documents, tallies.bodies, tallies.body_bytes))
}

fn check_top(archive: &Path, tallies: &mut Tallies) -> Result<(), ArchiveActivityReason> {
    let mut bodies = false;
    let mut captures = false;
    let entries = validate::read_archive_dir(archive)?;
    for entry in entries {
        tallies.visited = validate::bump(tallies.visited)?;
        let entry = entry.map_err(|_| ArchiveActivityReason::Io)?;
        let name = validate::entry_name(&entry)?;
        if name == "bodies" {
            bodies = true;
        } else if name == "captures" {
            captures = true;
        } else if name == "reservations" {
            validate::require_dir(&entry.path())?;
        } else {
            return Err(ArchiveActivityReason::UnexpectedLayout);
        }
    }
    if !bodies || !captures {
        return Err(ArchiveActivityReason::Missing);
    }
    validate::require_dir(&archive.join("bodies"))?;
    validate::require_dir(&archive.join("captures"))?;
    Ok(())
}

fn count_bodies(dir: &Path, tallies: &mut Tallies) -> Result<(), ArchiveActivityReason> {
    let entries = validate::read_archive_dir(dir)?;
    for entry in entries {
        tallies.visited = validate::bump(tallies.visited)?;
        let entry = entry.map_err(|_| ArchiveActivityReason::Io)?;
        let path = entry.path();
        let metadata = validate::regular_file(&path)?;
        let name = validate::entry_name(&entry)?;
        validate::body_digest(&name)?;
        let length = validate::body_length(&metadata)?;
        tallies.bodies = tallies
            .bodies
            .checked_add(1)
            .ok_or(ArchiveActivityReason::Limit)?;
        tallies.body_bytes = tallies
            .body_bytes
            .checked_add(length)
            .ok_or(ArchiveActivityReason::Limit)?;
    }
    Ok(())
}

fn count_captures(dir: &Path, tallies: &mut Tallies) -> Result<(), ArchiveActivityReason> {
    let entries = validate::read_archive_dir(dir)?;
    for entry in entries {
        tallies.visited = validate::bump(tallies.visited)?;
        let entry = entry.map_err(|_| ArchiveActivityReason::Io)?;
        let path = entry.path();
        validate::require_dir(&path)?;
        let name = validate::entry_name(&entry)?;
        validate::content_digest_name(&name)?;
        count_content(&path, &name, tallies)?;
    }
    Ok(())
}

fn count_content(
    dir: &Path,
    content: &str,
    tallies: &mut Tallies,
) -> Result<(), ArchiveActivityReason> {
    let entries = validate::read_archive_dir(dir)?;
    for entry in entries {
        tallies.visited = validate::bump(tallies.visited)?;
        let entry = entry.map_err(|_| ArchiveActivityReason::Io)?;
        let path = entry.path();
        let metadata = validate::regular_file(&path)?;
        let name = validate::entry_name(&entry)?;
        let stem = validate::meta_digest(&name)?;
        count_meta(&path, &metadata, content, stem, tallies)?;
    }
    Ok(())
}

fn count_meta(
    path: &Path,
    metadata: &std::fs::Metadata,
    content: &str,
    stem: &str,
    tallies: &mut Tallies,
) -> Result<(), ArchiveActivityReason> {
    let ceiling = u64::try_from(crate::net::cache::MAX_META_BYTES)
        .map_err(|_| ArchiveActivityReason::Limit)?;
    if metadata.len() > ceiling {
        return Err(ArchiveActivityReason::Limit);
    }
    tallies.meta_bytes = tallies
        .meta_bytes
        .checked_add(metadata.len())
        .ok_or(ArchiveActivityReason::Limit)?;
    if tallies.meta_bytes > MAX_ACTIVITY_META_TOTAL_BYTES {
        return Err(ArchiveActivityReason::Limit);
    }
    let bytes =
        match crate::net::cache::read_cache_file(path, crate::net::cache::MAX_META_BYTES, None) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Err(ArchiveActivityReason::Corrupt),
            Err(_) => return Err(ArchiveActivityReason::Io),
        };
    let capture = crate::net::cache::Capture::from_bytes(&bytes, path)
        .map_err(|_| ArchiveActivityReason::Corrupt)?;
    if capture.meta.content_digest != content {
        return Err(ArchiveActivityReason::Corrupt);
    }
    if bytes != capture.encoded {
        return Err(ArchiveActivityReason::Corrupt);
    }
    if crate::net::cache::content_digest(&capture.encoded) != stem {
        return Err(ArchiveActivityReason::Corrupt);
    }
    tallies.documents = tallies
        .documents
        .checked_add(1)
        .ok_or(ArchiveActivityReason::Limit)?;
    Ok(())
}
