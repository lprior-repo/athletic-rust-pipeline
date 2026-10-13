use census_domain::model::{ProfileSelectionV1, QueryProfileInputV1, QueryProfileV1};
use census_store::MAX_PROFILE_RECORD_BYTES;
use serde::{de::DeserializeOwned, Deserialize};
use std::fmt;
use std::io::Read;
use std::path::Path;

use super::error::{ProfileCliError, ProfileCliResult};

#[derive(Debug, Clone, Copy)]
pub(super) enum InputKind {
    Profile,
    Selection,
}

impl fmt::Display for InputKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Profile => "query profile",
            Self::Selection => "profile selection",
        })
    }
}

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

pub(super) fn read_profile(path: &Path) -> ProfileCliResult<QueryProfileV1> {
    parse_profile(&read_file(path)?)
}

pub(super) fn read_selection(path: &Path) -> ProfileCliResult<ProfileSelectionV1> {
    parse_versioned(&read_file(path)?, InputKind::Selection)
}

pub(super) fn parse_profile(bytes: &[u8]) -> ProfileCliResult<QueryProfileV1> {
    let input: QueryProfileInputV1 = parse_versioned(bytes, InputKind::Profile)?;
    QueryProfileV1::new(input).map_err(ProfileCliError::InvalidProfile)
}

pub(super) fn parse_versioned<T: DeserializeOwned>(
    bytes: &[u8],
    kind: InputKind,
) -> ProfileCliResult<T> {
    check_size(u64::try_from(bytes.len()).map_err(|_| ProfileCliError::InputSizeArithmetic)?)?;
    let header: VersionHeader = parse_json(bytes, kind)?;
    if header.version != 1 {
        return Err(ProfileCliError::InputVersion {
            kind,
            found: header.version,
        });
    }
    parse_json(bytes, kind)
}

fn parse_json<T: DeserializeOwned>(bytes: &[u8], kind: InputKind) -> ProfileCliResult<T> {
    serde_json::from_slice(bytes).map_err(|source| ProfileCliError::InputJson { kind, source })
}

fn read_file(path: &Path) -> ProfileCliResult<Vec<u8>> {
    let metadata = std::fs::metadata(path).map_err(|source| ProfileCliError::InputMetadata {
        path: path.to_path_buf(),
        source,
    })?;
    regular_file(path, &metadata)?;
    check_size(metadata.len())?;
    let mut file = std::fs::File::open(path).map_err(|source| ProfileCliError::InputOpen {
        path: path.to_path_buf(),
        source,
    })?;
    let opened = file
        .metadata()
        .map_err(|source| ProfileCliError::InputMetadata {
            path: path.to_path_buf(),
            source,
        })?;
    regular_file(path, &opened)?;
    check_size(opened.len())?;
    read_limited(&mut file)
}

fn regular_file(path: &Path, metadata: &std::fs::Metadata) -> ProfileCliResult<()> {
    if !metadata.is_file() {
        return Err(ProfileCliError::InputNotFile {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn check_size(observed: u64) -> ProfileCliResult<()> {
    let max = u64::try_from(MAX_PROFILE_RECORD_BYTES)
        .map_err(|_| ProfileCliError::InputSizeArithmetic)?;
    if observed > max {
        return Err(ProfileCliError::InputTooLarge {
            observed,
            max: MAX_PROFILE_RECORD_BYTES,
        });
    }
    Ok(())
}

pub(super) fn read_limited<R: Read>(reader: &mut R) -> ProfileCliResult<Vec<u8>> {
    let capacity = MAX_PROFILE_RECORD_BYTES
        .checked_add(1)
        .ok_or(ProfileCliError::InputSizeArithmetic)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(ProfileCliError::AllocationFailed)?;
    bytes.resize(capacity, 0);
    let mut used = 0_usize;
    for _ in 0..capacity {
        let target = bytes
            .get_mut(used..)
            .ok_or(ProfileCliError::InputSizeArithmetic)?;
        let read = reader.read(target).map_err(ProfileCliError::InputRead)?;
        if read == 0 {
            bytes.truncate(used);
            return Ok(bytes);
        }
        used = used
            .checked_add(read)
            .ok_or(ProfileCliError::InputSizeArithmetic)?;
        check_size(u64::try_from(used).map_err(|_| ProfileCliError::InputSizeArithmetic)?)?;
    }
    Err(ProfileCliError::InputReadBudget)
}
