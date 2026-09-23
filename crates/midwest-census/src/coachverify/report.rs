use crate::coachverify::verdict::{FragmentOutcome, RowOutcome};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Render the per-fragment audit table as markdown rows (header included).
pub fn audit_table(outcomes: &[FragmentOutcome]) -> String {
    let mut table = String::from(
        "| fragment | rows | verified | role conveyed by page context | role contradicted | render-required | mismatch | empty | robots-blocked | fetch failed | shipped share |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    let mut totals: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut rows_total = 0usize;
    for outcome in outcomes {
        let count = |verdict: super::verdict::Verdict| {
            outcome.counts.get(verdict.as_str()).copied().unwrap_or(0)
        };
        let total = outcome.rows.len();
        let shipped = count(super::verdict::Verdict::Ok)
            .saturating_add(count(super::verdict::Verdict::OkRoleContext));
        rows_total = rows_total.saturating_add(total);
        for verdict in super::verdict::Verdict::ALL {
            let slot = totals.entry(verdict.as_str()).or_insert(0);
            *slot = slot.saturating_add(count(*verdict));
        }
        let share = pct(shipped, total);
        table.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} % |\n",
            outcome.file,
            total,
            count(super::verdict::Verdict::Ok),
            count(super::verdict::Verdict::OkRoleContext),
            count(super::verdict::Verdict::RoleContradicted),
            count(super::verdict::Verdict::RenderRequired),
            count(super::verdict::Verdict::Mismatch),
            count(super::verdict::Verdict::Empty),
            count(super::verdict::Verdict::RobotsBlocked),
            count(super::verdict::Verdict::FetchFailed),
            share
        ));
    }
    let shipped_total = totals
        .get("ok")
        .copied()
        .unwrap_or(0)
        .saturating_add(totals.get("ok_role_context").copied().unwrap_or(0));
    let share = pct(shipped_total, rows_total);
    table.push_str(&format!(
        "| **total ({})** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{:.1} %** |\n",
        outcomes.len(),
        rows_total,
        totals.get("ok").copied().unwrap_or(0),
        totals.get("ok_role_context").copied().unwrap_or(0),
        totals.get("role_contradicted").copied().unwrap_or(0),
        totals.get("render_required").copied().unwrap_or(0),
        totals.get("mismatch").copied().unwrap_or(0),
        totals.get("empty").copied().unwrap_or(0),
        totals.get("robots_blocked").copied().unwrap_or(0),
        totals.get("fetch_failed").copied().unwrap_or(0),
        share
    ));
    table
}

/// Compute a percentage safely: 0.0 when the denominator is zero.
fn pct(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        let numerator = u32::try_from(num).map_or(f64::MAX, f64::from);
        let denominator = u32::try_from(den).map_or(f64::MAX, f64::from);
        numerator * 100.0 / denominator
    }
}

/// Write the per-row verdict CSV (one row per fragment row, verdict included).
///
/// Publication is atomic: the audit csv is replaced whole, so a reader holding `path` never
/// observes a partial audit.
pub fn write_audit_csv(path: &Path, outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    crate::store::read::publish_atomically(path, |temporary| {
        write_audit_csv_body(temporary, path, outcomes)
    })?;
    Ok(())
}

/// Write the audit rows to `temporary`; publication renames it onto `published`.
fn write_audit_csv_body(
    temporary: &Path,
    published: &Path,
    outcomes: &[FragmentOutcome],
) -> crate::store::StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .from_path(temporary)
        .map_err(|error| crate::store::read::csv_failure(published, error))?;
    writer
        .write_record([
            "fragment",
            "state",
            "sport",
            "role",
            "school",
            "coach_name",
            "ad_name",
            "source_url",
            super::VERDICT_COLUMN,
        ])
        .map_err(|error| crate::store::read::csv_failure(published, error))?;
    for outcome in outcomes {
        for row in &outcome.rows {
            writer
                .write_record([
                    outcome.file.as_str(),
                    row.row.state.as_str(),
                    row.row.sport.as_str(),
                    row.row.role.as_str(),
                    row.row.school.as_str(),
                    row.row.coach_name.as_str(),
                    row.row.ad_name.as_str(),
                    row.row.source_urls.join(" ").as_str(),
                    row.verdict.as_str(),
                ])
                .map_err(|error| crate::store::read::csv_failure(published, error))?;
        }
    }
    writer.flush().map_err(|source| crate::store::StoreError::Io {
        path: published.to_path_buf(),
        source,
    })
}

/// Group verified rows by state and write one `<ST>.csv` per state into `dir` — the shape
/// `merge-coaches` consumes, because that step reads a directory whose file stems are state codes.
/// Rows keep their provenance columns and the recomputed verdict; the merge step dedupes them.
///
/// Returns the row count staged per state.
pub fn write_state_union(
    dir: &Path,
    outcomes: &[FragmentOutcome],
) -> anyhow::Result<BTreeMap<String, usize>> {
    use anyhow::Context;
    std::fs::create_dir_all(dir).with_context(|| format!("create staging dir {dir:?}"))?;
    let mut by_state: BTreeMap<String, Vec<&RowOutcome>> = BTreeMap::new();
    for outcome in outcomes {
        for row in outcome.shipped() {
            by_state
                .entry(row.row.state.trim().to_ascii_uppercase())
                .or_default()
                .push(row);
        }
    }
    let mut counts = BTreeMap::new();
    for (state, rows) in by_state {
        if state.len() != 2 || !state.is_ascii() {
            tracing::warn!("skipping {state}: not a two-letter state code");
            continue;
        }
        let path = dir.join(format!("{state}.csv"));
        crate::store::read::publish_atomically(&path, |temporary| {
            write_state_file_body(temporary, &path, &rows)
        })?;
        counts.insert(state, rows.len());
    }
    Ok(counts)
}

/// Write one state's staged fragment to `temporary`; publication renames it onto `published`.
///
/// `merge-coaches` reads `<ST>.csv` by these eleven columns; a twelfth column makes it reject the
/// whole state as unusable.
fn write_state_file_body(
    temporary: &Path,
    published: &Path,
    rows: &[&RowOutcome],
) -> crate::store::StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .from_path(temporary)
        .map_err(|error| crate::store::read::csv_failure(published, error))?;
    writer
        .write_record(super::FRAGMENT_COLUMNS)
        .map_err(|error| crate::store::read::csv_failure(published, error))?;
    for row in rows {
        writer
            .write_record([
                row.row.school.as_str(),
                row.row.city.as_str(),
                row.row.state.as_str(),
                row.row.sport.as_str(),
                row.row.role.as_str(),
                row.row.coach_name.as_str(),
                row.row.public_professional_email.as_str(),
                row.row.ad_name.as_str(),
                row.row.ad_email.as_str(),
                row.row.source_urls.join(" ").as_str(),
                row.row.last_observed.as_str(),
            ])
            .map_err(|error| crate::store::read::csv_failure(published, error))?;
    }
    writer.flush().map_err(|source| crate::store::StoreError::Io {
        path: published.to_path_buf(),
        source,
    })
}

/// Write the freeze manifest: the timestamp, one `sha256 <path>` line per input fragment, then one
/// verdict line per fragment. It ties every tally in the audit doc to the exact input bytes.
pub fn write_manifest(
    path: &Path,
    files: &[PathBuf],
    outcomes: &[FragmentOutcome],
) -> anyhow::Result<()> {
    use anyhow::Context;
    use sha2::{Digest, Sha256};
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create manifest dir {parent:?}"))?;
    }
    let mut manifest = String::new();
    manifest.push_str(&crate::net::now_iso8601());
    manifest.push('\n');
    let mut sorted: Vec<&PathBuf> = files.iter().collect();
    sorted.sort();
    for file in sorted {
        let bytes =
            std::fs::read(file).with_context(|| format!("read fragment {file:?} for hashing"))?;
        let digest = Sha256::digest(&bytes);
        manifest.push_str(&format!("{digest:x}  {}\n", file.display()));
    }
    manifest.push('\n');
    for outcome in outcomes {
        manifest.push_str(&outcome.log_line());
        manifest.push('\n');
    }
    std::fs::write(path, manifest).with_context(|| format!("write manifest {path:?}"))?;
    Ok(())
}

/// Every host the given fragment files cite, lowercased and deduplicated. A caller that treats the
/// operator's commission of this collection as authorization for those hosts passes this list to the
/// fetcher, which then records each such request as `robots_authorized` under its pacing ceiling.
pub fn cited_hosts(files: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    use anyhow::Context;
    static URL_HOST: LazyLock<Option<regex::Regex>> =
        LazyLock::new(|| regex::Regex::new(r#"https?://([^/\s,"']+)"#).ok());
    let Some(url_host) = URL_HOST.as_ref() else {
        return Ok(Vec::new());
    };
    let mut hosts = BTreeSet::new();
    for file in files {
        let bytes =
            std::fs::read(file).with_context(|| format!("read fragment {file:?} for its hosts"))?;
        let text = String::from_utf8_lossy(&bytes);
        for capture in url_host.captures_iter(&text) {
            if let Some(host) = capture.get(1) {
                hosts.insert(host.as_str().to_ascii_lowercase());
            }
        }
    }
    Ok(hosts.into_iter().collect())
}
