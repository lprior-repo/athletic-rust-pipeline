use anyhow::{Context, Result};

mod configuration;
mod files;
pub(in super::super) mod fixture;
mod journal;
mod marker;

fn failure<T>(result: Result<T>) -> Result<String> {
    Ok(format!(
        "{:#}",
        result.err().context("expected native boundary refusal")?
    ))
}
