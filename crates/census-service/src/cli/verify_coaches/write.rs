use anyhow::{bail, Context, Result};
use census_crawl::net::FetchStats;
use census_service::coachverify::{self, FragmentOutcome};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Expand the arguments into a sorted list of fragment CSVs: directories contribute their `*.csv`.
pub fn collect_fragments(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            for entry in path
                .read_dir()
                .with_context(|| format!("read fragment directory {path:?}"))?
            {
                let entry = entry?;
                let candidate = entry.path();
                if candidate.extension().is_some_and(|ext| ext == "csv") {
                    files.push(candidate);
                }
            }
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            bail!("fragment path {path:?} is neither a file nor a directory");
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Write all output artifacts and print results.
pub async fn print_results(
    args: &super::VerifyCoachesArgs,
    outcomes: &[FragmentOutcome],
    files: &[PathBuf],
    fetcher: &census_crawl::net::Fetcher,
    failures: &[anyhow::Error],
    out_dir: &Path,
) -> Result<()> {
    write_log(&args.log, outcomes)?;
    print_row_logs(outcomes);
    write_outputs(args, outcomes, files)?;
    print_reconciliation(&args.reconcile, outcomes);
    let shipped = compute_shipped(outcomes);
    let stats = fetcher.stats().await;
    print_fetch_stats(&stats);
    print_verification_summary(outcomes, shipped, out_dir);
    check_failures(failures)
}

/// Append tally lines to the freeze log.
fn write_log(log: &Option<PathBuf>, outcomes: &[FragmentOutcome]) -> Result<()> {
    if let Some(log) = log {
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .with_context(|| format!("open freeze log {log:?}"))?;
        for outcome in outcomes {
            writeln!(handle, "{}", outcome.log_line())?;
        }
        handle.flush()?;
    }
    Ok(())
}

/// Print one tally line per outcome to stdout.
fn print_row_logs(outcomes: &[FragmentOutcome]) {
    for outcome in outcomes {
        println!("{}", outcome.log_line());
    }
}

/// Write all output artifacts: CSV, union, manifest, report.
fn write_outputs(
    args: &super::VerifyCoachesArgs,
    outcomes: &[FragmentOutcome],
    files: &[PathBuf],
) -> Result<()> {
    if let Some(report) = &args.report {
        write_report(report, outcomes)?;
    }
    if let Some(csv) = &args.csv {
        coachverify::write_audit_csv(csv, outcomes)?;
    }
    if let Some(union) = &args.union {
        let staged = coachverify::write_state_union(union, outcomes)?;
        let total: usize = staged.values().sum();
        println!(
            "staged {total} verified rows in {} state files -> {}",
            staged.len(),
            union.display()
        );
    }
    if let Some(manifest) = &args.manifest {
        coachverify::write_manifest(manifest, files, outcomes)?;
        println!("manifest: {}", manifest.display());
    }
    Ok(())
}

/// Print reconciliation output against a published artifact.
fn print_reconciliation(published: &Option<PathBuf>, outcomes: &[FragmentOutcome]) {
    if let Some(published) = published {
        let reconciliation = coachverify::reconcile(published, outcomes).unwrap_or_default();
        println!(
            "verified rows: {} distinct identities from {} fragment files",
            reconciliation.verified, reconciliation.files
        );
        println!(
            "merged rows:   {}  ({})",
            reconciliation.published,
            published.display()
        );
        println!(
            "merged rows with no verified counterpart: {}",
            reconciliation.unmatched_total()
        );
        for (state, count) in &reconciliation.unmatched {
            println!("  {state}: {count}");
        }
    }
}

/// Count shipped rows across all outcomes.
fn compute_shipped(outcomes: &[FragmentOutcome]) -> usize {
    outcomes
        .iter()
        .map(|outcome| outcome.shipped().count())
        .sum()
}

/// Print fetch stats from the HTTP fetcher.
fn print_fetch_stats(stats: &FetchStats) {
    println!(
        "fetch stats: requests={} cache_hits={} robots_blocked={} robots_authorized={} errors={} bytes={}",
        stats.requests,
        stats.cache_hits,
        stats.robots_blocked,
        stats.robots_authorized,
        stats.errors,
        stats.bytes_downloaded
    );
}

/// Print the verification summary line.
fn print_verification_summary(outcomes: &[FragmentOutcome], shipped: usize, out_dir: &Path) {
    println!(
        "verified fragments: {} files, {} rows, {} shipped -> {}",
        outcomes.len(),
        outcomes
            .iter()
            .map(|outcome| outcome.rows.len())
            .sum::<usize>(),
        shipped,
        out_dir.display()
    );
}

/// Report fragment-level failures, or return Ok.
fn check_failures(failures: &[anyhow::Error]) -> Result<()> {
    if !failures.is_empty() {
        let messages: Vec<String> = failures.iter().map(|error| format!("{error:#}")).collect();
        bail!(
            "{} fragment(s) failed to verify: {}",
            failures.len(),
            messages.join("; ")
        );
    }
    Ok(())
}

/// Write the audit table plus the tally block that surrounds it in the published audit doc.
fn write_report(path: &Path, outcomes: &[FragmentOutcome]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create report dir {parent:?}"))?;
    }
    let mut body = String::new();
    body.push_str("\n## Classification of every row\n\n");
    body.push_str(
        "Every row below was re-fetched from its own `source_url` cells. `verified` means a value\n\
         cell appeared in the page *and* the role label was corroborated within its window;\n\
         `role conveyed by page context` means the value appeared without that window; `render-required`\n\
         means the page is a JavaScript shell, so no value is reachable without executing script.\n\
         `robots-blocked` and `fetch failed` count pages the polite fetcher could not read at all —\n\
         those rows ship nowhere and the tallies say how many they are.\n\n",
    );
    body.push_str(&coachverify::audit_table(outcomes));
    std::fs::write(path, body).with_context(|| format!("write report {path:?}"))?;
    Ok(())
}
