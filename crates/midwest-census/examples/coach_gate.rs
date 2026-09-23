//! Throwaway driver for the coach-fragment gate while `cli/verify_coaches.rs`'s package is being
//! edited concurrently by the compile-fix lane. Runs the same library entry points the subcommand
//! calls, so the numbers it prints are the subcommand's numbers.
//!
//! Usage: cargo run --release -p midwest-census --example coach_gate -- <out_dir> <jobs> <fragment...>
//! Env: AUTHORIZED_HOSTS (comma-separated), RECONCILE (published csv), LOG (freeze log), REPORT (md),
//!      CSV (per-row verdicts), DELAY_MS, REFRESH=1.

use futures::stream::{self, StreamExt};
use midwest_census::coachverify::{self, FragmentOutcome, GateOptions};
use midwest_census::net::Fetcher;
use midwest_census::sources;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The gate's positional arguments: where verified fragments land, how many run at once, and the
/// fragment files themselves.
struct Args {
    out_dir: PathBuf,
    jobs: usize,
    fragments: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let Args {
        out_dir,
        jobs,
        fragments,
    } = parse_args()?;
    let fetcher = build_fetcher(&fragments)?;
    let options = gate_options();
    let outcomes = verify_fragments(&fetcher, &fragments, &out_dir, &options, jobs).await?;
    emit_freeze_log(&outcomes)?;
    for outcome in &outcomes {
        println!("{}", outcome.log_line());
    }
    write_reports(&outcomes)?;
    write_manifest(&fragments, &outcomes)?;
    reconcile_published(&outcomes)?;
    print_summary(&fetcher, &outcomes, &out_dir).await;
    Ok(())
}

/// Install the `tracing` subscriber: `RUST_LOG` when it parses, `info` otherwise.
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
}

/// Read `<out_dir> <jobs> <fragment...>`, defaulting the first two, and refuse an empty fragment list
/// — a gate with no inputs has nothing to say.
fn parse_args() -> anyhow::Result<Args> {
    let mut args = std::env::args().skip(1);
    let out_dir = PathBuf::from(args.next().unwrap_or_else(|| "out/verified".to_string()));
    let jobs: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8);
    let fragments: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if fragments.is_empty() {
        anyhow::bail!("usage: coach_gate <out_dir> <jobs> <fragment...>");
    }
    Ok(Args {
        out_dir,
        jobs,
        fragments,
    })
}

/// Build the fetcher the gate reads citations through: `CACHE_DIR` holds the bodies, `DELAY_MS` is
/// the per-host delay, and the allow-list is whatever the citations (or `AUTHORIZED_HOSTS`) imply.
fn build_fetcher(fragments: &[PathBuf]) -> anyhow::Result<Fetcher> {
    let cache_dir = PathBuf::from(
        std::env::var("CACHE_DIR").unwrap_or_else(|_| "var/midwest-census/http".to_string()),
    );
    let authorized = authorized_hosts(fragments)?;
    let delay_ms: u64 = std::env::var("DELAY_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1000);
    let fetcher = Fetcher::new(
        &cache_dir,
        None,
        Duration::from_millis(delay_ms),
        sources::default_host_delays(),
        authorized,
    )?
    .with_family_budgets(sources::default_family_delays());
    Ok(fetcher)
}

/// The host allow-list: every host the fragments cite when `AUTHORIZE_CITED` is set (the count is
/// reported, because that decision changes what the gate may fetch), otherwise the comma-separated
/// `AUTHORIZED_HOSTS` entries.
fn authorized_hosts(fragments: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    if std::env::var("AUTHORIZE_CITED").is_ok() {
        let hosts = midwest_census::coachverify::cited_hosts(fragments)?;
        println!("authorizing {} cited hosts", hosts.len());
        return Ok(hosts);
    }
    Ok(std::env::var("AUTHORIZED_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .map(str::to_string)
        .collect())
}

/// Every pass the gate runs, all enabled: the XHR + Referer pass, the NSAA form POST, `pdftotext`
/// for PDF citations, and `REFRESH=1` to ignore cached bodies.
fn gate_options() -> GateOptions {
    GateOptions {
        xhr_pass: true,
        nsaa_post: true,
        pdftotext: Some("pdftotext".to_string()),
        refresh: std::env::var("REFRESH").is_ok(),
    }
}

/// Re-derive every fragment from its own cited pages, at most `jobs` in flight, then sort by file so
/// the report does not depend on completion order.
async fn verify_fragments(
    fetcher: &Fetcher,
    fragments: &[PathBuf],
    out_dir: &Path,
    options: &GateOptions,
    jobs: usize,
) -> anyhow::Result<Vec<FragmentOutcome>> {
    let results: Vec<anyhow::Result<FragmentOutcome>> =
        stream::iter(fragments.iter().cloned())
            .map(|path| async move {
                coachverify::verify_fragment(fetcher, &path, out_dir, options).await
            })
            .buffer_unordered(jobs.max(1))
            .collect()
            .await;
    let mut outcomes = Vec::new();
    for result in results {
        outcomes.push(result?);
    }
    outcomes.sort_by(|left, right| left.file.cmp(&right.file));
    Ok(outcomes)
}

/// Append one tally line per fragment to the freeze log `LOG` names.
fn emit_freeze_log(outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    if let Ok(log) = std::env::var("LOG") {
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)?;
        for outcome in outcomes {
            writeln!(handle, "{}", outcome.log_line())?;
        }
        handle.flush()?;
    }
    Ok(())
}

/// Write the `REPORT` audit table, the `CSV` per-row verdicts and the `UNION` per-state files.
fn write_reports(outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    if let Ok(report) = std::env::var("REPORT") {
        std::fs::write(&report, coachverify::audit_table(outcomes))?;
        println!("report: {report}");
    }
    if let Ok(csv) = std::env::var("CSV") {
        coachverify::write_audit_csv(&PathBuf::from(&csv), outcomes)?;
        println!("csv: {csv}");
    }
    if let Ok(union) = std::env::var("UNION") {
        let union = PathBuf::from(&union);
        let staged = coachverify::write_state_union(&union, outcomes)?;
        println!(
            "staged {} verified rows in {} state files -> {}",
            staged.values().sum::<usize>(),
            staged.len(),
            union.display()
        );
    }
    Ok(())
}

/// Write the `MANIFEST` freeze manifest over the input fragments and their verdicts.
fn write_manifest(fragments: &[PathBuf], outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    if let Ok(manifest) = std::env::var("MANIFEST") {
        let manifest = PathBuf::from(&manifest);
        coachverify::write_manifest(&manifest, fragments, outcomes)?;
        println!("manifest: {}", manifest.display());
    }
    Ok(())
}

/// Reconcile the `RECONCILE` published csv against the verified rows, printing what the published
/// artifact holds that no verified fragment backs.
fn reconcile_published(outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    if let Ok(published) = std::env::var("RECONCILE") {
        let reconciliation = coachverify::reconcile(&PathBuf::from(&published), outcomes)?;
        println!(
            "verified rows: {} distinct identities from {} fragment files",
            reconciliation.verified, reconciliation.files
        );
        println!("merged rows:   {}  ({published})", reconciliation.published);
        println!(
            "merged rows with no verified counterpart: {}",
            reconciliation.unmatched_total()
        );
        for (state, count) in &reconciliation.unmatched {
            println!("  {state}: {count}");
        }
    }
    Ok(())
}

/// The closing numbers: what the fetcher did, and what shipped.
async fn print_summary(fetcher: &Fetcher, outcomes: &[FragmentOutcome], out_dir: &Path) {
    let stats = fetcher.stats().await;
    println!(
        "fetch stats: requests={} cache_hits={} robots_blocked={} robots_authorized={} errors={} bytes={}",
        stats.requests,
        stats.cache_hits,
        stats.robots_blocked,
        stats.robots_authorized,
        stats.errors,
        stats.bytes_downloaded
    );
    println!(
        "verified fragments: {} files, {} rows, {} shipped -> {}",
        outcomes.len(),
        outcomes.iter().map(|o| o.rows.len()).sum::<usize>(),
        outcomes.iter().map(|o| o.shipped().count()).sum::<usize>(),
        out_dir.display()
    );
}
