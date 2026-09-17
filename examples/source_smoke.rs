use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::{
    domain::{evidence::Sport, identity::EvidenceDigest},
    search::{parse_page, SearchQuery},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
};

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("expected public search response path")?;
    let mut body = Vec::new();
    File::open(path)?
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut body)?;
    ensure!(
        body.len() <= 32 * 1024 * 1024,
        "response exceeds source boundary"
    );
    let digest = EvidenceDigest::parse(&format!("{:x}", Sha256::digest(&body)))?;
    let query = SearchQuery::new("Grant Fisher", Sport::TrackField, 0)?;
    let page = parse_page(&query, 0, digest, &body)?;
    let exact_names = page
        .candidates
        .iter()
        .filter(|candidate| {
            candidate
                .display_name
                .split_whitespace()
                .eq(["Grant", "Fisher"])
        })
        .count();
    ensure!(
        exact_names > 0,
        "public positive control did not yield the requested athlete name"
    );
    ensure!(
        page.issues.is_empty(),
        "public response has parse issues: {:?}",
        page.issues
    );
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(
        &mut output,
        &serde_json::json!({
            "public_source_only": true,
            "reported_count": page.count,
            "parsed_candidates": page.candidates.len(),
            "exact_public_name_candidates": exact_names,
            "issues": page.issues.len(),
            "next_offset": page.next_offset,
        }),
    )?;
    output.write_all(b"\n")?;
    Ok(())
}
