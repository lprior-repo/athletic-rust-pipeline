use anyhow::{Context, Result};

mod configuration;
mod files;
mod fixture;
mod journal;
mod marker;

fn failure<T>(result: Result<T>) -> Result<String> {
    Ok(format!(
        "{:#}",
        result.err().context("expected native boundary refusal")?
    ))
}
