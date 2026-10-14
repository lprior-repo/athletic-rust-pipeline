use std::env;

use super::TestResult;

const MAX_ROWS: u64 = 2_000_000;
const MAX_PAYLOAD: usize = 16 * 1024;
const MAX_INPUT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub(super) const BATCH_ROWS: u64 = 256;

pub(super) struct Profile {
    pub(super) name: &'static str,
    pub(super) rows: u64,
    pub(super) payload: String,
    pub(super) input_preflight: u64,
}

impl Profile {
    pub(super) fn read() -> TestResult<Self> {
        let rows = optional("CENSUS_SOURCE_CAPTURE_ROWS")?;
        let payload = optional("CENSUS_SOURCE_CAPTURE_PAYLOAD_BYTES")?;
        Self::from_inputs(rows.as_deref(), payload.as_deref())
    }

    fn from_inputs(rows: Option<&str>, payload: Option<&str>) -> TestResult<Self> {
        let (name, rows, payload) = match (rows, payload) {
            (None, None) => ("synthetic_small_default", 2000, 64),
            (Some(rows), Some(payload)) => (
                "synthetic_explicit_override",
                rows.parse()?,
                payload.parse()?,
            ),
            _ => return Err("set both source capture rows and payload bytes, or neither".into()),
        };
        let input_preflight = preflight(rows, payload)?;
        Ok(Self {
            name,
            rows,
            payload: "x".repeat(payload),
            input_preflight,
        })
    }
}

fn optional(name: &str) -> TestResult<Option<String>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn preflight(rows: u64, payload: usize) -> TestResult<u64> {
    if rows == 0 || rows > MAX_ROWS || payload > MAX_PAYLOAD {
        return Err("synthetic source capture profile exceeds row/payload limits".into());
    }
    let record_bytes = u64::try_from(payload)?
        .checked_add(1024)
        .ok_or("record bytes overflow")?;
    let input_bytes = rows
        .checked_mul(record_bytes)
        .ok_or("input bytes overflow")?;
    if input_bytes > MAX_INPUT_BYTES {
        return Err("synthetic source capture profile exceeds 4-GiB input preflight".into());
    }
    Ok(input_bytes)
}

pub(super) fn admit<T>(
    read: impl FnOnce() -> TestResult<Profile>,
    create_store: impl FnOnce(&Profile) -> TestResult<T>,
) -> TestResult<(Profile, T)> {
    let profile = read()?;
    let store = create_store(&profile)?;
    Ok((profile, store))
}

#[path = "config_tests.rs"]
mod tests;
