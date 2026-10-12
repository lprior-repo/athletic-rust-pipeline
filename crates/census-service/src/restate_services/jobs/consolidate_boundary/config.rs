use std::path::PathBuf;
use std::time::Duration;

use super::{BoundaryError, BOUNDARY_ENVIRONMENT, HOLD_ENVIRONMENT, MARKER_ENVIRONMENT};

const DEFAULT_HOLD_MILLIS: u64 = 60_000;
const MAX_HOLD_MILLIS: u64 = 60_000;

pub(super) fn armed() -> Result<Option<String>, BoundaryError> {
    match std::env::var(BOUNDARY_ENVIRONMENT) {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(BoundaryError::Arming(error.to_string())),
        Ok(value) if value.is_empty() => Err(BoundaryError::Arming(
            "the boundary environment names no operation".to_string(),
        )),
        Ok(value) => Ok(Some(value)),
    }
}

pub(super) fn marker_path() -> Result<PathBuf, BoundaryError> {
    let value = std::env::var_os(MARKER_ENVIRONMENT).ok_or_else(|| {
        BoundaryError::Marker("the armed boundary requires an absolute marker path".to_string())
    })?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(BoundaryError::Marker(format!(
            "{} is not absolute",
            path.display()
        )));
    }
    Ok(path)
}

pub(super) fn hold_millis() -> Result<u64, BoundaryError> {
    let value = match std::env::var(HOLD_ENVIRONMENT) {
        Err(std::env::VarError::NotPresent) => return Ok(DEFAULT_HOLD_MILLIS),
        Err(error) => return Err(BoundaryError::HoldConfig(error.to_string())),
        Ok(value) => value,
    };
    let millis = value
        .parse::<u64>()
        .map_err(|error| BoundaryError::HoldConfig(format!("{value:?}: {error}")))?;
    if millis == 0 || millis > MAX_HOLD_MILLIS {
        return Err(BoundaryError::Hold(millis));
    }
    Ok(millis)
}

pub(super) fn deliver(operation: &str, millis: u64) -> Result<(), BoundaryError> {
    std::thread::sleep(Duration::from_millis(millis));
    Err(BoundaryError::HoldExpired {
        operation: operation.to_string(),
        millis,
    })
}
