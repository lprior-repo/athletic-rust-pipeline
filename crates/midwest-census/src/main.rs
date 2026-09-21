//! `midwest-census` CLI. Every subcommand is safe to re-run: HTTP bodies are cached, finished work
//! is journaled in the Fjall store, and observations are append-only, so an interrupted walk
//! continues where it stopped.
//!
//! One process owns the store at a time: the database takes an exclusive lock in
//! [`Store::open`](midwest_census::store::Store::open), so a run either holds the store for its
//! whole life or fails with the reason instead of interleaving writes with another process.

#![forbid(unsafe_code)]

mod cli;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}
