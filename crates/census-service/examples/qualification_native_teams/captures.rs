use anyhow::{ensure, Context, Result};
use census_crawl::milesplit::{parse_team_index, Site};
use census_domain::UsJurisdiction;
use census_store::Store;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

use super::artifacts::{write_json, write_new};

const INDEX: &[u8] =
    include_bytes!("../../../census-crawl/tests/fixtures/milesplit/oh_teams_index.html");

pub fn prepare(root: &Path) -> Result<()> {
    let jurisdiction = UsJurisdiction::from_code("OH").context("OH jurisdiction missing")?;
    let text = std::str::from_utf8(INDEX)?;
    let teams = parse_team_index(text)?;
    ensure!(
        !teams.is_empty(),
        "authentic Ohio index contains no parsed teams"
    );
    let store = Store::open(root.join("store"))?;
    let url = Site::for_jurisdiction(jurisdiction).teams_url();
    let key_digest = Sha256::digest(format!("GET\u{1f}{url}\u{1f}").as_bytes());
    let key = hex(key_digest
        .get(..16)
        .context("cache digest prefix missing")?);
    let digest = hex(&Sha256::digest(INDEX));
    let installed =
        chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now()).to_rfc3339();
    let metadata = json!({"url":url, "method":"GET", "status":200, "content_digest":digest, "bytes":INDEX.len(), "fetched_at":installed, "content_type":"text/html"});
    write_new(&store.http_cache_dir().join(format!("{key}.body")), INDEX)?;
    write_json(
        &store.http_cache_dir().join(format!("{key}.meta.json")),
        &metadata,
    )?;
    write_new(&root.join("authentic-oh-teams-index.html"), INDEX)?;
    store.flush()?;
    drop(store);
    write_json(
        &root.join("capture-preparation.json"),
        &json!({"source":"crates/census-crawl/tests/fixtures/milesplit/oh_teams_index.html", "sha256":digest, "bytes":INDEX.len(), "url":url, "parsed_teams":teams.len(), "metadata_time_semantics":"qualification cache installation time; original source capture time not asserted", "fresh_acquisition":false, "athlete_population_seeded":false, "facts_seeded":0, "native_state_seeded":false, "store_flushed_and_closed_before_endpoint":true, "purpose":"teams refresh bypasses this capture; production rosters independently reads the authentic index with refresh=false; no stage success is seeded"}),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
