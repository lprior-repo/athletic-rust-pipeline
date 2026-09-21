//! Committed measurement for `Runtime::blocking` fan-out at `cpu_workers` 1, 8 and 32.
//!
//! ```text
//! cargo bench -p athletic-rust-pipeline --bench blocking_fanout
//! ```
//!
//! **Why N=64 is not benchable in this process.** The Phase 7 plan asks for a fetch-concurrency
//! curve at N ∈ {1, 8, 64}; no root-crate path admits 64 concurrent in-process fetches:
//!
//! * `browser.tabs` is capped at 1..=8 (`BrowserSettings::validate`, `src/runtime/browser.rs:30`;
//!   `RawBrowserConfig::validate`, `src/runtime/browser_config.rs:35`), and the browser fetch
//!   pool's queue is `tabs * QUEUE_MULTIPLIER(4)` = 32 outstanding jobs at most
//!   (`src/runtime/browser/actor.rs:27`, applied at `:114`; the same multiplier sizes the channel
//!   in `src/runtime/browser/lifecycle/startup.rs:22`).
//! * `cpu_workers` is capped at 1..=32 (`src/runtime/config.rs:139`), so 64 CPU-bound blocking
//!   actions cannot even be admitted by the semaphore this bench measures.
//! * `row_concurrency` is capped at 1..=128 (`src/runtime/config.rs:142`), but it bounds durable
//!   Restate row invocations rather than in-process fan-out.
//! * The end-to-end browser fetch path cannot be benched in-repo at all: `browser::transport::fetch`
//!   (`src/runtime/browser/transport.rs:75`) needs a live CDP session, and no in-process harness
//!   provides one.
//!
//! Line numbers are as of 2026-09-21 (HEAD fd9c8e3 plus the in-flight `src/runtime/**` refactors);
//! the symbol names above are the stable reference if the lines drift again.
//!
//! Raising any of those caps is an owner decision, not a benchmark's, so this target measures the
//! widest fan-out the root crate actually admits (32) and records the curve below it.
//!
//! **What is measured.** `TASKS` concurrent `Runtime::blocking` actions per iteration — the bounded
//! `spawn_blocking` pool every CPU-bound action goes through (`Runtime::blocking`,
//! `src/runtime/lifecycle.rs:88`, with `spawn_blocking` at `:99` and the `cpu_workers` semaphore at
//! `:98`), reported as jobs/s per configured worker count. Each action is a bounded, allocation-free, dependent
//! multiply chain, so the curve shows the pool's real admission behaviour instead of spawn cost.
//!
//! **Self-asserting counts.** Before any rate is reported, every worker count is exercised once and
//! checked: all `TASKS` actions must complete, their results must be the exact deterministic values
//! the action body produces, and the observed number of concurrently *executing* blocking actions
//! must never exceed the configured `cpu_workers`. Each measured iteration re-checks completion
//! count and the cap, so a run that dropped an action or admitted more concurrency than configured
//! fails instead of reporting a rate for it.

use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::runtime::Runtime;
use criterion::{Criterion, Throughput};
use futures::{StreamExt, TryStreamExt};
use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tempfile::TempDir;

/// Blocking actions requested per measured fan-out.
const TASKS: usize = 256;
/// Dependent multiply-adds per action: bounded, allocation-free, ~1 ms per action on a modern core.
const WORK_ITERATIONS: u64 = 1 << 20;
/// LCG multiplier (Numerical Recipes) for the deterministic action body.
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
/// Worker counts the root configuration admits for `cpu_workers`.
const CPU_WORKERS: [usize; 3] = [1, 8, 32];

fn main() {
    let dataset = Dataset::build()
        .unwrap_or_else(|error| panic!("blocking_fanout dataset is not usable: {error:#}"));
    let mut criterion = Criterion::default().configure_from_args();
    for workers in CPU_WORKERS {
        bench_workers(&mut criterion, &dataset, workers);
    }
    criterion.final_summary();
}

/// One configured worker runtime per measured worker count, each on its own temporary store.
struct Dataset {
    _directory: TempDir,
    runtime: tokio::runtime::Runtime,
    workers: Vec<(usize, Arc<Runtime>)>,
}

impl Dataset {
    fn build() -> Result<Self> {
        let directory = tempfile::tempdir().context("creating the bench directory")?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .build()
            .context("building the bench tokio runtime")?;
        let mut workers = Vec::with_capacity(CPU_WORKERS.len());
        for count in CPU_WORKERS {
            let storage = directory.path().join(format!("store-{count}"));
            let config = directory.path().join(format!("worker-{count}.toml"));
            write_config(&config, &storage, count)?;
            let worker = open_runtime(&runtime, &config)?;
            ensure!(
                worker.config.cpu_workers() == count,
                "the worker runtime reports {} cpu_workers, expected {count}",
                worker.config.cpu_workers()
            );
            let peak = verify_fan_out(&runtime, &worker, count)?;
            println!(
                "fan-out verified: cpu_workers={count} tasks={TASKS} \
                 peak_concurrent_actions={peak} value_checks={TASKS}"
            );
            workers.push((count, worker));
        }
        println!("metric=bench_fanout_tasks value={TASKS} unit=actions");
        println!("metric=bench_work_iterations value={WORK_ITERATIONS} unit=iterations");
        Ok(Self {
            _directory: directory,
            runtime,
            workers,
        })
    }

    /// The runtime configured with `count` cpu workers.
    fn worker(&self, count: usize) -> Result<&Arc<Runtime>> {
        self.workers
            .iter()
            .find(|(workers, _)| *workers == count)
            .map(|(_, runtime)| runtime)
            .context("the bench runtime for this worker count was not built")
    }
}

/// `Runtime::blocking` fan-out at one worker count.
fn bench_workers(criterion: &mut Criterion, dataset: &Dataset, workers: usize) {
    let runtime = checked(dataset.worker(workers));
    let bench_runtime = &dataset.runtime;
    let mut group = criterion.benchmark_group(format!("blocking_fanout/cpu_workers_{workers}"));
    group.throughput(Throughput::Elements(row_count(TASKS)));
    group.bench_function("blocking_actions", |bencher| {
        bencher.iter(|| {
            let progress = Arc::new(Progress::default());
            let values = checked(bench_runtime.block_on(fan_out(runtime, &progress)));
            require(
                values.len() == TASKS,
                "the measured fan-out did not complete every blocking action",
            );
            require(
                progress.peak.load(Ordering::SeqCst) <= workers,
                "the measured fan-out exceeded the configured cpu_workers cap",
            );
            std::hint::black_box(values);
        })
    });
    group.finish();
}

/// Run the fan-out once and check every count, value and cap it claims.
fn verify_fan_out(
    runtime: &tokio::runtime::Runtime,
    worker: &Arc<Runtime>,
    workers: usize,
) -> Result<usize> {
    let progress = Arc::new(Progress::default());
    let mut produced = runtime.block_on(fan_out(worker, &progress))?;
    ensure!(
        produced.len() == TASKS,
        "the fan-out completed {} actions, expected {TASKS}",
        produced.len()
    );
    let mut wanted = (0..TASKS).map(expected).collect::<Vec<_>>();
    produced.sort_unstable();
    wanted.sort_unstable();
    ensure!(
        produced == wanted,
        "the fan-out did not run every action to its deterministic result"
    );
    let peak = progress.peak.load(Ordering::SeqCst);
    ensure!(
        peak > 0,
        "the fan-out observed no concurrent blocking action at all"
    );
    ensure!(
        peak <= workers,
        "the fan-out reached {peak} concurrent blocking actions, the cap is {workers}"
    );
    Ok(peak)
}

/// `TASKS` actions through `Runtime::blocking`, each entering and leaving the concurrency counter
/// inside the blocking action so the count measures execution, not queueing.
async fn fan_out(runtime: &Arc<Runtime>, progress: &Arc<Progress>) -> Result<Vec<u64>> {
    futures::stream::iter(0..TASKS)
        .map(|index| {
            let seed = u64::try_from(index).unwrap_or(u64::MAX);
            let counter = Arc::clone(progress);
            async move {
                runtime
                    .blocking(move || {
                        counter.enter();
                        let value = work(seed);
                        counter.leave();
                        Ok(value)
                    })
                    .await
                    .context("blocking action failed")
            }
        })
        .buffer_unordered(TASKS)
        .try_collect::<Vec<_>>()
        .await
}

/// Concurrently executing blocking actions, and the peak observed.
#[derive(Default)]
struct Progress {
    in_flight: AtomicUsize,
    peak: AtomicUsize,
}

impl Progress {
    /// Record one action entering the blocking pool, keeping the peak.
    fn enter(&self) {
        let now = self
            .in_flight
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        self.peak.fetch_max(now, Ordering::SeqCst);
    }

    /// Record one action leaving it.
    fn leave(&self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Bounded, deterministic, allocation-free CPU action for one blocking job: a dependent multiply
/// chain, so the cost per action is stable and every result is a checkable value.
fn work(seed: u64) -> u64 {
    let mut state = seed;
    for index in 0..WORK_ITERATIONS {
        state = state.wrapping_mul(MULTIPLIER).wrapping_add(index);
    }
    state
}

/// The value the action for `index` must produce.
fn expected(index: usize) -> u64 {
    work(u64::try_from(index).unwrap_or(u64::MAX))
}

/// A worker configuration with `workers` cpu workers, one absolute temporary storage directory,
/// and the loopback fixture origin the config validator requires.
fn write_config(path: &Path, storage: &Path, workers: usize) -> Result<()> {
    let document = format!(
        "mode = \"fixture\"\n\
         storage_dir = {storage:?}\n\
         source_origin = \"http://127.0.0.1:8081/\"\n\
         source_interval_ms = 1000\n\
         request_timeout_seconds = 30\n\
         cpu_workers = {workers}\n\
         row_concurrency = 1\n\
         q5_url = \"http://127.0.0.1:8082/\"\n\
         q5_model = \"bench-q5\"\n\
         q4_url = \"http://127.0.0.1:8083/\"\n\
         q4_model = \"bench-q4\"\n"
    );
    fs::write(path, document).with_context(|| format!("writing {}", path.display()))
}

/// Open a worker runtime from inside the bench tokio runtime's context (`Runtime::open` builds an
/// HTTP client, which needs a reactor to exist).
fn open_runtime(runtime: &tokio::runtime::Runtime, config: &Path) -> Result<Arc<Runtime>> {
    let entered = runtime.enter();
    let worker = Runtime::open(config);
    drop(entered);
    worker.with_context(|| format!("opening a worker runtime from {}", config.display()))
}

/// A count as the `u64` a criterion throughput declaration needs.
fn row_count(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Fail the measurement when the workload itself fails rather than reporting a rate for it.
fn checked<T, E>(result: std::result::Result<T, E>) -> T
where
    E: std::fmt::Display,
{
    result.unwrap_or_else(|error| panic!("blocking fan-out workload failed: {error}"))
}

/// Fail the measurement when an iteration did not do what the reported rate would claim.
fn require(condition: bool, message: &str) {
    if !condition {
        panic!("{message}");
    }
}
