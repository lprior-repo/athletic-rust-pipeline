#![cfg_attr(test, allow(dead_code))]
mod summary;
mod commands;
mod alpha_api;
mod alpha_api_client;
mod alpha_api_client_validation;
mod alpha_catalog;
mod alpha_checkpoint;
mod alpha_cohort;
mod alpha_config;
mod alpha_merge;
mod alpha_model;
mod alpha_model_raw;
mod alpha_model_raw_validation;
mod alpha_nav_validation;
mod alpha_normalize;
mod alpha_normalize_helpers;
mod alpha_output; mod alpha_output_privacy;
mod alpha_pipeline; mod alpha_pipeline_records; mod alpha_match; mod alpha_route_validation;
mod alpha_url;
#[cfg(test)] mod alpha_model_validation_tests;
#[cfg(test)] mod alpha_api_tests;
#[cfg(test)] mod alpha_api_client_regression_tests;
#[cfg(test)] mod alpha_api_client_incomplete_regression_tests;
#[cfg(test)] mod alpha_api_client_async_tests;
#[cfg(test)] mod alpha_api_client_validation_tests;
#[cfg(test)] mod alpha_api_client_cap_marker_tests;
#[cfg(test)] mod alpha_api_client_validation_regression_tests;
#[cfg(test)] mod alpha_api_client_nav_tests;
#[cfg(test)] mod alpha_api_client_nav_info_tests;
#[cfg(test)] mod alpha_api_completeness_tests;
#[cfg(test)] mod alpha_api_completeness_nav_tests;
#[cfg(test)] mod alpha_api_completeness_enforce_tests;
#[cfg(test)] mod alpha_model_tests;
#[cfg(test)] mod alpha_config_auth_tests;
#[cfg(test)] mod alpha_config_api_tests;
#[cfg(test)] mod alpha_config_loading_tests;
#[cfg(test)] mod alpha_config_test_helpers;
#[cfg(test)] mod alpha_config_pagination_tests;
#[cfg(test)] mod alpha_config_route_tests;
#[cfg(test)] mod alpha_test_helpers;
#[cfg(test)] mod alpha_nav_validation_tests;
#[cfg(test)] mod alpha_model_raw_validation_negative_season_tests;
#[cfg(test)] mod alpha_api_client_pagination_tests;
#[cfg(test)] mod alpha_api_client_pagination_config_tests;
#[cfg(test)] mod alpha_api_client_constructor_tests;
#[cfg(test)] mod alpha_api_deserialization_tests;
#[cfg(test)] mod alpha_api_field_validation_tests;
#[cfg(test)] mod alpha_catalog_matrix_tests;
#[cfg(test)] mod alpha_catalog_validation_tests;
#[cfg(test)] mod alpha_catalog_nav_tests;
#[cfg(test)] mod alpha_cohort_tests; #[cfg(test)] mod alpha_normalize_regression_tests;
#[cfg(test)] mod alpha_output_regression_tests; #[cfg(test)] mod alpha_normalize_tests;
#[cfg(test)] mod alpha_merge_tests;
#[cfg(test)] mod alpha_merge_tests_part2;
mod checkpoint;
mod config;
mod discovery;
mod extract;
mod fetch;
mod marks;
mod model;
mod output;
mod scoring;
mod search_cache;
mod xlsx;
use anyhow::{Context, Result}; use clap::Parser;
use config::Config;
use commands::Command;
use model::{MatchRecord, ModelDecision};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { input } => summary::inspect(&input),
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
        Command::CollectAuthorized {
            alpha_config,
            out_dir,
            max_units,
            i_have_alpha_authorization,
        } => {
            let summary = alpha_pipeline::collect_authorized(
                &alpha_config,
                &out_dir,
                max_units,
                i_have_alpha_authorization,
            )
            .await?;
            eprintln!(
                "authorized alpha collection complete: {} athletes, {} cohort exceptions, {} unresolved; output {}",
                summary.athlete_count,
                summary.cohort_exception_count,
                summary.unresolved_count,
                out_dir.display()
            );
            Ok(())
        }
        Command::MatchAuthorized { input, alpha_source, config, out_dir, max } =>
            alpha_match::match_workbook(&input, &alpha_source, &config, &out_dir, max).await,
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
    let scan = xlsx::scan(
        input,
        xlsx::ScanMode::Sports(sports),
        config.workbook.expected_graduation_year,
    )?;
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
                    let mut html = if let Some(ref dir) = config.retrieval.saved_pages_dir {
                        fetch::load_saved_profile(&hit.url, dir)?
                    } else {
                        None
                    };
                    if html.is_none() {
                        match fetch::fetch_exact_profile(&hit.url, &config.retrieval).await {
                            Ok(fetched) => html = Some(fetched),
                            Err(e) => eprintln!("  retrieval failed for {}: {e}", hit.url),
                        }
                    }
                    if let Some(ref html) = html {
                        let mut enriched = extract::candidate_from_evidence(
                            prospect,
                            hit,
                            Some(html.as_str()),
                            &ollama,
                            config.retrieval.page_text_limit,
                        )
                        .await;
                        scoring::score_candidate(prospect, &mut enriched, &config.matching);
                        if let Some(slot) = candidates.get_mut(selected_index) {
                            *slot = enriched;
                        }
                    }
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
            .map_or(0, |d| d.as_secs());

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
    summary::summarize(&ordered);
    Ok(())
}
