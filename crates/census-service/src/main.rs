#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

mod cli;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
