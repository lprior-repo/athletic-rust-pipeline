use census_crawl as sources;
use census_crawl::net::Fetcher;
use census_service::coachverify::{self, FragmentOutcome, GateOptions};
use futures::stream::{self, StreamExt};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
        println!("{}", log_line(outcome));
    }
    write_reports(&outcomes)?;
    write_manifest(&fragments, &outcomes)?;
    reconcile_published(&outcomes)?;
    print_summary(&fetcher, &outcomes, &out_dir).await;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
}

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

fn build_fetcher(fragments: &[PathBuf]) -> anyhow::Result<Fetcher> {
    let cache_dir = PathBuf::from(
        std::env::var("CACHE_DIR").unwrap_or_else(|_| "var/census-service/http".to_string()),
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

fn authorized_hosts(fragments: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    if std::env::var("AUTHORIZE_CITED").is_ok() {
        let hosts = census_service::coachverify::cited_hosts(fragments)?;
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

fn gate_options() -> GateOptions {
    GateOptions {
        xhr_pass: true,
        nsaa_post: true,
        pdftotext: Some("pdftotext".to_string()),
        refresh: std::env::var("REFRESH").is_ok(),
    }
}

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

fn log_line(outcome: &FragmentOutcome) -> String {
    let verdicts = outcome
        .counts
        .iter()
        .map(|(verdict, count)| format!("{verdict:?}={count}"))
        .collect::<Vec<String>>()
        .join(",");
    format!("{} rows={} {verdicts}", outcome.file, outcome.rows.len())
}

fn emit_freeze_log(outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    if let Ok(log) = std::env::var("LOG") {
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)?;
        for outcome in outcomes {
            writeln!(handle, "{}", log_line(outcome))?;
        }
        handle.flush()?;
    }
    Ok(())
}

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

fn write_manifest(fragments: &[PathBuf], outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    if let Ok(manifest) = std::env::var("MANIFEST") {
        let manifest = PathBuf::from(&manifest);
        let union = std::env::var("UNION").ok().map(PathBuf::from);
        coachverify::write_manifest(&manifest, fragments, outcomes, union.as_deref())?;
        println!("manifest: {}", manifest.display());
    }
    Ok(())
}

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

async fn print_summary(fetcher: &Fetcher, outcomes: &[FragmentOutcome], out_dir: &Path) {
    let stats = fetcher.stats().await;
    println!(
        "fetch stats: requests={} cache_hits={} errors={} bytes={}",
        stats.requests,
        stats.cache_hits,
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
