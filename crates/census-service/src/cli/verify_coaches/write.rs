use anyhow::{bail, Context, Result};
use census_crawl::net::FetchStats;
use census_service::coachverify::{self, FragmentOutcome};
use std::io::Write;
use std::path::{Path, PathBuf};

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
    print_reconciliation(&args.reconcile, outcomes)?;
    let shipped = compute_shipped(outcomes);
    let stats = fetcher.stats().await;
    print_fetch_stats(&stats);
    print_verification_summary(outcomes, shipped, out_dir);
    check_failures(failures)
}

fn write_log(log: &Option<PathBuf>, outcomes: &[FragmentOutcome]) -> Result<()> {
    if let Some(log) = log {
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .with_context(|| format!("open freeze log {log:?}"))?;
        for outcome in outcomes {
            serde_json::to_writer(&mut handle, &outcome.summary())?;
            writeln!(handle)?;
        }
        handle.flush()?;
    }
    Ok(())
}

fn print_row_logs(outcomes: &[FragmentOutcome]) {
    for outcome in outcomes {
        tracing::info!(fragment = %outcome.file, rows = outcome.rows.len(),
            verdicts = ?outcome.counts, "contact fragment inspected");
    }
}

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
        coachverify::write_manifest(manifest, files, outcomes, args.union.as_deref())?;
        tracing::info!(path = %manifest.display(), "contact manifest written");
    }
    Ok(())
}

fn print_reconciliation(published: &Option<PathBuf>, outcomes: &[FragmentOutcome]) -> Result<()> {
    if let Some(published) = published {
        let reconciliation = coachverify::reconcile(published, outcomes)?;
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
    Ok(())
}

fn compute_shipped(outcomes: &[FragmentOutcome]) -> usize {
    outcomes
        .iter()
        .map(|outcome| outcome.shipped().count())
        .sum()
}

fn print_fetch_stats(stats: &FetchStats) {
    println!(
        "fetch stats: requests={} cache_hits={} errors={} bytes={}",
        stats.requests, stats.cache_hits, stats.errors, stats.bytes_downloaded
    );
}

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

fn write_report(path: &Path, outcomes: &[FragmentOutcome]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create report dir {parent:?}"))?;
    }
    let mut body = String::new();
    body.push_str("\n## Classification of every row\n\n");
    body.push_str(
        "Every row below was re-fetched from its own `source_url` cells. `verified` means a value\n\
         cell appeared in the page *and* the role label was corroborated within its window;\n\
         `role conveyed by page context` means the value appeared without that window;\n\
         `attribution-required` means the page literally carries the row's own school and claimed\n\
         person but no role-context window tied them to a claim, so the gap is attribution, not\n\
         rendering; `render-required` means the page is a JavaScript shell that does not carry the\n\
         person, so no value is reachable without executing script; `mismatch` means the page\n\
         carries neither the row's school nor its claimed person. `fetch failed` counts pages the\n\
         fetcher could not read at all —\n\
         those rows ship nowhere and the tallies say how many they are.\n\n",
    );
    body.push_str(&coachverify::audit_table(outcomes));
    std::fs::write(path, body).with_context(|| format!("write report {path:?}"))?;
    Ok(())
}
