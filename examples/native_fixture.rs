#![forbid(unsafe_code)]

mod native_fixture {
    pub mod payloads;
    pub mod scenarios;
    pub mod server;
    pub mod workbook;
}

use anyhow::{Context, Result};
use clap::Parser;
use native_fixture::{
    scenarios::{Scenario, ScenarioSet},
    server::state,
    workbook::generate,
};
use std::{net::SocketAddr, path::PathBuf, str::FromStr};

#[derive(Debug, Parser)]
#[command(
    name = "native_fixture",
    about = "Synthetic HTTP and XLSX fixture for the native Restate pipeline"
)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:18081", value_name = "IP:PORT")]
    bind: SocketAddr,
    #[arg(long, default_value = "fixture-output", value_name = "DIR")]
    output_dir: PathBuf,
    #[arg(long, default_value = "all", value_name = "NAME[,NAME...]")]
    scenario: String,
    #[arg(long, default_value_t = 0)]
    delay_ms: u64,
    #[arg(long, default_value_t = 16)]
    max_concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if !args.bind.ip().is_loopback() || args.bind.port() == 0 {
        anyhow::bail!("fixture requires a loopback address and an explicit nonzero port");
    }
    let output_dir = absolute(&args.output_dir)?;
    let scenarios = parse_scenarios(&args.scenario)?;
    let workbook = generate(&output_dir, &scenarios)?;
    let config = write_worker_config(&output_dir, args.bind)?;
    println!(
        "{}",
        serde_json::json!({
            "bind": format!("http://{}/", args.bind),
            "workbook": workbook,
            "worker_config": config,
            "scenarios": scenarios.scenarios.iter().map(|value| value.as_str()).collect::<Vec<_>>(),
            "controls": {"counters": format!("http://{}/__fixture/counters", args.bind), "control": format!("http://{}/__fixture/control", args.bind), "reset": format!("http://{}/__fixture/reset", args.bind)},
            "model_ids": ["fixture-q5-5090", "fixture-q4-3090"],
        })
    );
    native_fixture::server::serve(
        state(scenarios, args.delay_ms, args.max_concurrency),
        args.bind,
    )
    .await
}

fn absolute(path: &std::path::Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn parse_scenarios(raw: &str) -> Result<ScenarioSet> {
    if raw.trim().eq_ignore_ascii_case("all") {
        return Ok(ScenarioSet::all());
    }
    let scenarios = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Scenario::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(anyhow::Error::msg)?;
    if scenarios.is_empty() {
        anyhow::bail!("--scenario must name at least one fixture case");
    }
    Ok(ScenarioSet { scenarios })
}

fn write_worker_config(output_dir: &std::path::Path, bind: SocketAddr) -> Result<PathBuf> {
    let storage = output_dir.join("artifacts");
    let path = output_dir.join("worker.toml");
    let origin = format!("http://{bind}/");
    let text = format!("mode = \"fixture\"\nstorage_dir = \"{}\"\nsource_origin = \"{}\"\nsource_interval_ms = 0\nrequest_timeout_seconds = 30\ncpu_workers = 2\nrow_concurrency = 8\nq5_url = \"{}\"\nq5_model = \"fixture-q5-5090\"\nq4_url = \"{}\"\nq4_model = \"fixture-q4-3090\"\n", storage.display(), origin, origin, origin);
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}
