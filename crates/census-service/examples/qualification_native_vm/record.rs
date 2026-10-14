use super::artifacts;
use anyhow::{bail, ensure, Context, Result};
use census_crawl::milesplit::{parse_roster, parse_team_index, RosterVerdict, Site};
use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::UsJurisdiction;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub fn run(args: Args) -> Result<()> {
    let jurisdiction =
        UsJurisdiction::from_code(&args.jurisdiction).context("unknown jurisdiction code")?;
    let site = Site::for_jurisdiction(jurisdiction);
    let index_url = site.teams_url();
    let index_url_owned = index_url.to_string();
    let team_id_owned = args.team_id.clone();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating capture recorder runtime")?;

    let results = runtime.block_on(async {
        let fetcher = build_fetcher(&args.cache, args.source.as_deref())?;
        let index = acquire(&fetcher, &index_url_owned, "index").await?;
        let text = std::str::from_utf8(&index.body).context("index body not UTF-8")?;
        let parsed = parse_team_index(text)?;
        ensure!(
            parsed.disposition == census_crawl::CollectionDisposition::Complete,
            "team index is not complete: {:?}",
            parsed.disposition
        );
        let team = parsed
            .teams
            .iter()
            .find(|t| t.id == team_id_owned)
            .context("requested team not found in index")?;
        let roster_url = format!("{}/roster", team.url);
        let roster = acquire(&fetcher, &roster_url, "roster").await?;
        Ok::<_, anyhow::Error>((index, roster, roster_url))
    })?;

    let (index_outcome, roster_outcome, _roster_url) = results;

    let roster_text = std::str::from_utf8(&roster_outcome.body)?;
    let index_text = std::str::from_utf8(&index_outcome.body)?;
    let index_parsed = parse_team_index(index_text)?;
    let team = index_parsed
        .teams
        .iter()
        .find(|t| t.id == args.team_id)
        .context("requested team not found after acquisition")?;
    let verdict = parse_roster(roster_text, team.clone())?;
    match verdict {
        RosterVerdict::Complete { .. } => {}
        other => bail!("roster not complete: {:?}", other),
    }

    std::fs::create_dir_all(&args.cache).context("creating cache directory")?;
    let payload = args.cache.join("payload");
    std::fs::create_dir_all(&payload).context("creating payload directory")?;

    let index_body = payload.join("index.body");
    let index_meta = payload.join("index.meta.json");
    let roster_body = payload.join("roster.body");
    let roster_meta = payload.join("roster.meta.json");

    artifacts::write(&index_body, &index_outcome.body)?;
    artifacts::publish(&index_meta, &outcome_meta(&index_outcome))?;
    artifacts::write(&roster_body, &roster_outcome.body)?;
    artifacts::publish(&roster_meta, &outcome_meta(&roster_outcome))?;

    ensure!(
        !args.out.exists(),
        "refusing to overwrite existing manifest at {}",
        args.out.display()
    );
    std::fs::create_dir_all(args.out.parent().context("manifest parent")?)
        .context("creating manifest parent")?;

    let manifest = json!({
        "jurisdiction": args.jurisdiction,
        "team_id": args.team_id,
        "index": {
            "body": index_body.display().to_string(),
            "metadata": index_meta.display().to_string()
        },
        "roster": {
            "body": roster_body.display().to_string(),
            "metadata": roster_meta.display().to_string()
        }
    });
    artifacts::publish(&args.out, &manifest)?;

    let absolute = args
        .out
        .canonicalize()
        .context("canonicalizing manifest path")?;
    println!("{}", absolute.display());
    Ok(())
}

fn build_fetcher(cache: &std::path::Path, source: Option<&str>) -> Result<Fetcher> {
    let mut builder = Fetcher::new(
        cache.join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )
    .context("building fetcher")?;
    if let Some(src) = source {
        builder = builder.with_source(src);
    }
    Ok(builder)
}

async fn acquire(
    fetcher: &Fetcher,
    url: &str,
    label: &str,
) -> Result<census_crawl::net::FetchOutcome> {
    let options = FetchOptions {
        refresh: true,
        ..Default::default()
    };
    let outcome = fetcher
        .get(url, &options)
        .await
        .with_context(|| format!("fetching {label} from {url}"))?;
    ensure!(
        !outcome.from_cache,
        "{label} response was served from cache, not live acquisition"
    );
    ensure!(
        outcome.status == 200,
        "{label} response status was {} not 200",
        outcome.status
    );
    ensure!(!outcome.body.is_empty(), "{label} body was empty");
    Ok(outcome)
}

fn outcome_meta(outcome: &census_crawl::net::FetchOutcome) -> Value {
    json!({
        "url": outcome.url,
        "content_digest": outcome.content_digest,
        "status": outcome.status,
        "fetched_at": outcome.fetched_at,
        "bytes": outcome.bytes,
        "content_type": outcome.content_type,
        "from_cache": outcome.from_cache
    })
}

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub jurisdiction: String,
    #[arg(long)]
    pub team_id: String,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long)]
    pub cache: PathBuf,
    #[arg(long)]
    pub source: Option<String>,
}
