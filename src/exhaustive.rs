use crate::{
    checkpoint,
    config::Config,
    coverage,
    exhaustive_engine::Engine,
    model::{MatchRecord, Prospect},
    output, summary, xlsx,
};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::task::{JoinError, JoinHandle};

#[path = "exhaustive_run_rows.rs"]
mod exhaustive_run_rows;
#[path = "reuse_search.rs"]
mod reuse_search;

#[derive(Clone)]
pub struct RunOptions {
    pub max: Option<usize>,
    pub first_worksheet_only: bool,
    pub authorization_ack: bool,
    pub no_ai: bool,
    pub reuse_searches_from: Option<PathBuf>,
}

struct RunState {
    prospects: Vec<Prospect>,
    completed: HashMap<String, MatchRecord>,
    pending_indices: Vec<usize>,
    progress: coverage::CoverageReport,
    fingerprint: String,
    out_dir: PathBuf,
    _lock: File,
}

pub async fn run(
    input: &Path,
    config_path: &Path,
    out_dir: &Path,
    options: RunOptions,
) -> Result<()> {
    let paths = (input.to_owned(), config_path.to_owned(), out_dir.to_owned());
    let no_ai = options.no_ai;
    let prepared =
        tokio::task::spawn_blocking(move || prepare(&paths.0, &paths.1, &paths.2, options));
    let (mut state, config) = await_blocking("workbook preparation", prepared).await?;
    eprintln!(
        "Selected {} rows; fingerprint {}",
        state.prospects.len(),
        state.fingerprint
    );
    if state.pending_indices.is_empty() {
        return state.finalize().await;
    }

    let engine_out_dir = state.out_dir.clone();
    let engine = tokio::task::spawn_blocking(move || Engine::new(config, &engine_out_dir, no_ai));
    let mut engine = match await_blocking("engine initialization", engine).await {
        Ok(engine) => engine,
        Err(error) => {
            let finalization = state.finalize().await;
            return combine_results(Err(error), finalization);
        }
    };

    let processing = exhaustive_run_rows::process_rows(&mut engine, &mut state).await;
    let finalization = state.finalize().await;
    combine_results(processing, finalization)
}

fn combine_results(primary: Result<()>, finalization: Result<()>) -> Result<()> {
    match (primary, finalization) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(finalization_error)) => Err(error.context(format!(
            "run finalization also failed: {finalization_error:#}"
        ))),
    }
}

async fn await_blocking<T>(task_name: &'static str, handle: JoinHandle<Result<T>>) -> Result<T> {
    match handle.await {
        Ok(result) => result.with_context(|| format!("{task_name} task returned an error")),
        Err(error) => Err(classify_join_error(task_name, error)),
    }
}

fn classify_join_error(task_name: &str, error: JoinError) -> anyhow::Error {
    if error.is_cancelled() {
        anyhow::anyhow!("{task_name} task was cancelled before completion: {error}")
    } else if error.is_panic() {
        anyhow::anyhow!("{task_name} task panicked: {error}")
    } else {
        anyhow::anyhow!("{task_name} task failed to join: {error}")
    }
}

fn prepare(
    input: &Path,
    config_path: &Path,
    out_dir: &Path,
    options: RunOptions,
) -> Result<(RunState, Config)> {
    let config = Config::load(config_path)?;
    if config.retrieval.authorized_direct_fetch && !options.authorization_ack {
        anyhow::bail!("direct retrieval also requires --i-have-written-authorization");
    }
    std::fs::create_dir_all(out_dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(out_dir.join("run.lock"))?;
    lock.try_lock()
        .context("another process owns this output directory")?;
    let (scope, mode) = if options.first_worksheet_only {
        ("first-worksheet", xlsx::ScanMode::FirstWorksheet)
    } else {
        ("all-source-worksheets", xlsx::ScanMode::Exhaustive)
    };
    let scan = xlsx::scan(input, mode, config.workbook.expected_graduation_year)?;
    if scan.prospects.is_empty() {
        anyhow::bail!("No prospects found in workbook");
    }
    let donor_scope = format!("{scope}:deterministic");
    let scope = format!(
        "{scope}:{}",
        if options.no_ai {
            "deterministic"
        } else {
            "ai-review"
        }
    );
    let fingerprint = coverage::bind_run(input, config_path, out_dir, &scope)?;
    if let Some(donor) = &options.reuse_searches_from {
        reuse_search::import(input, config_path, out_dir, donor, &donor_scope)?;
    }
    let completed = checkpoint::load_latest(&out_dir.join("checkpoint.jsonl"))?;
    let progress = coverage::build_report(&fingerprint, &scan.prospects, &completed)?;
    let limit = options
        .max
        .map_or(scan.prospects.len(), |n| n.min(scan.prospects.len()));
    let pending_indices = scan
        .prospects
        .iter()
        .enumerate()
        .take(limit)
        .filter(|(_, prospect)| !coverage::is_final(&prospect.source_key, &completed))
        .map(|(index, _)| index)
        .collect();
    Ok((
        RunState {
            prospects: scan.prospects,
            completed,
            pending_indices,
            progress,
            fingerprint,
            out_dir: out_dir.to_owned(),
            _lock: lock,
        },
        config,
    ))
}

impl RunState {
    async fn commit(&mut self, mut record: MatchRecord) -> Result<()> {
        record.processed_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_secs();
        let checkpoint_path = self.out_dir.join("checkpoint.jsonl");
        let record = tokio::task::spawn_blocking(move || {
            checkpoint::append(&checkpoint_path, &record)?;
            Ok(record)
        });
        let record = await_blocking("checkpoint commit", record).await?;
        let failed = matches!(record.status.as_str(), "SEARCH_ERROR" | "AI_ERROR");
        let reason = if failed {
            Some(record.ai_logic.clone())
        } else {
            None
        };
        let progress = self
            .progress
            .record_committed(self.completed.get(&record.source_key), &record);
        self.completed.insert(record.source_key.clone(), record);
        progress?;
        if self.completed.len().is_multiple_of(100) {
            eprintln!("Committed {} rows", self.completed.len());
            self.write_coverage().await?;
        }
        if let Some(reason) = reason {
            anyhow::bail!("Retryable row failure: {reason}");
        }
        Ok(())
    }

    async fn finalize(self) -> Result<()> {
        let task = tokio::task::spawn_blocking(move || self.finalize_blocking());
        await_blocking("run finalization", task).await
    }

    fn finalize_blocking(mut self) -> Result<()> {
        let coverage_result = coverage::write_coverage(
            &self.out_dir,
            &self.fingerprint,
            &self.prospects,
            &self.completed,
        );
        let ordered = self
            .prospects
            .iter()
            .filter_map(|prospect| self.completed.remove(&prospect.source_key))
            .collect::<Vec<_>>();
        let export_result = output::write_all(&self.out_dir, &ordered);
        if export_result.is_ok() {
            summary::summarize(&ordered);
        }
        combine_results(coverage_result, export_result)
    }

    async fn write_coverage(&self) -> Result<()> {
        let out_dir = self.out_dir.clone();
        let report = self.progress.clone();
        let task = tokio::task::spawn_blocking(move || coverage::write_report(&out_dir, &report));
        await_blocking("coverage write", task).await
    }
}
