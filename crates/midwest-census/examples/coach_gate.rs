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
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
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
    let cache_dir = PathBuf::from(
        std::env::var("CACHE_DIR").unwrap_or_else(|_| "var/midwest-census/http".to_string()),
    );
    let authorized: Vec<String> = if std::env::var("AUTHORIZE_CITED").is_ok() {
        let hosts = midwest_census::coachverify::cited_hosts(&fragments)?;
        println!("authorizing {} cited hosts", hosts.len());
        hosts
    } else {
        std::env::var("AUTHORIZED_HOSTS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|host| !host.is_empty())
            .map(str::to_string)
            .collect()
    };
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
    let options = GateOptions {
        xhr_pass: true,
        nsaa_post: true,
        pdftotext: Some("pdftotext".to_string()),
        refresh: std::env::var("REFRESH").is_ok(),
    };
    let results: Vec<anyhow::Result<FragmentOutcome>> = stream::iter(fragments.iter().cloned())
        .map(|path| {
            let fetcher = &fetcher;
            let out_dir = out_dir.as_path();
            let options = &options;
            async move { coachverify::verify_fragment(fetcher, &path, out_dir, options).await }
        })
        .buffer_unordered(jobs.max(1))
        .collect()
        .await;
    let mut outcomes = Vec::new();
    for result in results {
        outcomes.push(result?);
    }
    outcomes.sort_by(|left, right| left.file.cmp(&right.file));
    if let Ok(log) = std::env::var("LOG") {
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)?;
        for outcome in &outcomes {
            writeln!(handle, "{}", outcome.log_line())?;
        }
        handle.flush()?;
    }
    for outcome in &outcomes {
        println!("{}", outcome.log_line());
    }
    if let Ok(report) = std::env::var("REPORT") {
        std::fs::write(&report, coachverify::audit_table(&outcomes))?;
        println!("report: {report}");
    }
    if let Ok(csv) = std::env::var("CSV") {
        coachverify::write_audit_csv(&PathBuf::from(&csv), &outcomes)?;
        println!("csv: {csv}");
    }
    if let Ok(union) = std::env::var("UNION") {
        let union = PathBuf::from(&union);
        let staged = coachverify::write_state_union(&union, &outcomes)?;
        println!(
            "staged {} verified rows in {} state files -> {}",
            staged.values().sum::<usize>(),
            staged.len(),
            union.display()
        );
    }
    if let Ok(manifest) = std::env::var("MANIFEST") {
        let manifest = PathBuf::from(&manifest);
        coachverify::write_manifest(&manifest, &fragments, &outcomes)?;
        println!("manifest: {}", manifest.display());
    }
    if let Ok(published) = std::env::var("RECONCILE") {
        let reconciliation = coachverify::reconcile(&PathBuf::from(&published), &outcomes)?;
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
    Ok(())
}
