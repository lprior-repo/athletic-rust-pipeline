#[cfg(test)]
#[allow(dead_code)]
mod alpha_config;
#[cfg(test)]
#[allow(dead_code)]
mod alpha_api;
#[cfg(test)]
mod alpha_api_client;
#[cfg(test)]
mod alpha_api_tests;
#[cfg(test)]
mod alpha_api_client_tests;
#[cfg(test)]
mod alpha_api_completeness_tests;
#[cfg(test)]
mod alpha_model;
#[cfg(test)]
mod alpha_route_validation;
#[cfg(test)]
mod alpha_config_validation_tests;
#[cfg(test)]
mod alpha_config_api_tests;
#[cfg(test)]
mod alpha_config_loading_tests;
#[cfg(test)]
mod alpha_model_tests;
mod checkpoint;
mod config;
mod discovery;
mod extract;
mod fetch;
mod marks;
mod model;
mod output;
mod scoring;
mod xlsx;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use model::{MatchRecord, ModelDecision};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Stream and report every real workbook row; performs no network access.
    Inspect {
        #[arg(long)]
        input: PathBuf,
    },
    /// Export every real source row to JSONL; performs no network access.
    ExportRecords {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Discover candidates, optionally retrieve pages, validate locally, and checkpoint.
    Run {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "out")]
        out_dir: PathBuf,
        #[arg(long)]
        max: Option<usize>,
        /// Include Cross Country rows in addition to configured sports.
        #[arg(long)]
        include_xc: bool,
        #[arg(long)]
        i_have_written_authorization: bool,
    },
    /// Add an Athletic Matches worksheet to a copy of the source workbook.
    Writeback {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        matches: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { input } => inspect(&input),
        Command::ExportRecords { input, output } => xlsx::export_records(&input, &output),
        Command::Run {
            input,
            config,
            out_dir,
            max,
            include_xc,
            i_have_written_authorization,
        } => {
            run_pipeline(
                &input,
                &config,
                &out_dir,
                max,
                include_xc,
                i_have_written_authorization,
            )
            .await
        }
        Command::Writeback {
            input,
            matches,
            output,
        } => {
            let records = output::read_jsonl(&matches)?;
            xlsx::append_matches_sheet(&input, &output, &records)
        }
    }
}

fn inspect(input: &Path) -> Result<()> {
    let result = xlsx::scan(input, &[], None)?;
    println!("{}", serde_json::to_string_pretty(&result.stats)?);
    Ok(())
}

async fn run_pipeline(
    input: &Path,
    config_path: &Path,
    out_dir: &Path,
    max: Option<usize>,
    include_xc: bool,
    authorization_ack: bool,
) -> Result<()> {
    let config = Config::load(config_path)?;
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output directory {}", out_dir.display()))?;

    if config.retrieval.authorized_direct_fetch && !authorization_ack {
        anyhow::bail!(
            "direct retrieval is enabled in config; also pass --i-have-written-authorization"
        );
    }

    let mut sports = config.workbook.sports.clone();
    if include_xc {
        for sport in ["Cross Country: Mens", "Cross Country: Womens"] {
            if !sports.iter().any(|configured| configured == sport) {
                sports.push(sport.to_owned());
            }
        }
    }
    let scan = xlsx::scan(input, &sports, config.workbook.expected_graduation_year)?;
    eprintln!(
        "parsed {} real rows; selected {} prospects",
        scan.stats.actual_data_rows,
        scan.prospects.len()
    );

    let checkpoint_path = out_dir.join("checkpoint.jsonl");
    let mut completed = checkpoint::load_latest(&checkpoint_path)?;
    let discovery = discovery::AthleticNetClient::new(&config.discovery)?;
    let ollama = extract::OllamaClient::new(&config.ollama)?;
    let prospect_end = max.map_or(scan.prospects.len(), |limit| {
        scan.prospects.len().min(limit)
    });
    let prospects = scan
        .prospects
        .get(..prospect_end)
        .context("prospect limit exceeded workbook scan bounds")?;

    for (index, prospect) in prospects.iter().enumerate() {
        if completed.contains_key(&prospect.source_key) {
            eprintln!(
                "[{}/{}] skip {} {}",
                index.saturating_add(1),
                prospects.len(),
                prospect.source_key,
                prospect.full_name()
            );
            continue;
        }

        eprintln!(
            "[{}/{}] discover {} | {}",
            index.saturating_add(1),
            prospects.len(),
            prospect.full_name(),
            prospect.school
        );

        let hits = match discovery.search(prospect).await {
            Ok(hits) => hits,
            Err(error) => {
                eprintln!("  discovery failed: {error:#}");
                continue;
            }
        };

        // Extract search evidence for every candidate first. This preserves all
        // candidate URLs and snippets while deferring network retrieval until
        // Rust has selected the strongest identity candidate.
        let mut candidates = Vec::with_capacity(hits.len());
        for hit in &hits {
            let mut candidate = extract::candidate_from_evidence(
                prospect,
                hit,
                None,
                &ollama,
                config.retrieval.page_text_limit,
            )
            .await;
            scoring::score_candidate(prospect, &mut candidate, &config.matching);
            candidates.push(candidate);
        }

        // Spider retrieves only the strongest candidate. All other candidates
        // remain available in the audit JSON/CSV with their search evidence.
        let selected_index = candidates
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.deterministic_score
                    .total_cmp(&right.deterministic_score)
            })
            .map(|(candidate_index, _)| candidate_index);
        if config.retrieval.authorized_direct_fetch {
            if let Some(selected_index) = selected_index {
                if let Some(hit) = hits.get(selected_index) {
                    let saved_html = match config.retrieval.saved_pages_dir.as_deref() {
                        Some(directory) => fetch::load_saved_profile(&hit.url, directory)?,
                        None => None,
                    };
                    let html = if saved_html.is_some() {
                        saved_html
                    } else {
                        match fetch::fetch_exact_profile(&hit.url, &config.retrieval).await {
                            Ok(html) => Some(html),
                            Err(error) => {
                                eprintln!("  retrieval failed for {}: {error:#}", hit.url);
                                None
                            }
                        }
                    };
                    if let Some(html) = html {
                        let mut enriched = extract::candidate_from_evidence(
                            prospect,
                            hit,
                            Some(&html),
                            &ollama,
                            config.retrieval.page_text_limit,
                        )
                        .await;
                        scoring::score_candidate(prospect, &mut enriched, &config.matching);
                        if let Some(slot) = candidates.get_mut(selected_index) {
                            *slot = enriched;
                        }
                    }
                } else {
                    eprintln!("  selected candidate index {selected_index} has no search hit");
                }
            }
        }

        let model_decision = if candidates.is_empty() {
            ModelDecision {
                decision: "NO_MATCH".to_owned(),
                model_status: "not_needed".to_owned(),
                reason: "No Athletic.net athlete candidate URL was discovered".to_owned(),
                ..Default::default()
            }
        } else {
            ollama.validate_identity(prospect, &candidates).await
        };

        let mut record = scoring::finalize_match(
            prospect.clone(),
            candidates,
            model_decision,
            &config.matching,
        );
        record.processed_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        checkpoint::append(&checkpoint_path, &record)?;
        eprintln!(
            "  => {} {:.3} {}",
            record.status, record.score, record.selected_profile_url
        );
        completed.insert(record.source_key.clone(), record);
    }

    let ordered: Vec<MatchRecord> = prospects
        .iter()
        .filter_map(|prospect| completed.get(&prospect.source_key).cloned())
        .collect();
    output::write_all(out_dir, &ordered)?;
    summarize(&ordered);
    Ok(())
}

fn summarize(records: &[MatchRecord]) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for record in records {
        counts
            .entry(record.status.as_str())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
    }
    eprintln!("wrote {} records", records.len());
    for status in ["MATCH", "CLOSE_MATCH", "REVIEW", "NO_MATCH"] {
        eprintln!(
            "  {status}: {}",
            counts.get(status).copied().map_or(0, |count| count)
        );
    }
}
