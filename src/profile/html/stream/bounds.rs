//! Capture, container, and depth bound enforcement for the streaming profile parse.

use super::buffer::StreamState;
use super::{MAX_CANDIDATES, MAX_CAPTURE_BYTES};
use anyhow::{bail, Context, Result};

impl StreamState {
    pub(super) fn push_candidate(output: &mut Vec<String>, value: String) -> Result<()> {
        if value.len() > MAX_CAPTURE_BYTES {
            bail!("profile identity URL exceeds capture bound");
        }
        if output.len() >= MAX_CANDIDATES {
            bail!("profile identity URL count exceeds bound");
        }
        output
            .try_reserve(1)
            .context("allocating identity URL evidence")?;
        output.push(value);
        Ok(())
    }

    pub(super) fn ensure_capacity(&self, current: usize, max: usize, label: &str) -> Result<()> {
        if current >= max {
            bail!("profile {label} exceeds bound");
        }
        Ok(())
    }
}

pub(super) fn append_bounded(output: &mut String, value: &str) -> Result<()> {
    let next = output
        .len()
        .checked_add(value.len())
        .context("profile capture byte count overflow")?;
    if next > MAX_CAPTURE_BYTES {
        bail!("profile capture exceeds bound");
    }
    output
        .try_reserve(value.len())
        .context("allocating profile capture")?;
    output.push_str(value);
    Ok(())
}
