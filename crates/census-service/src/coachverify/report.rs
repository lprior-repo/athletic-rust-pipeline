use crate::coachverify::evidence_path;
use crate::coachverify::verdict::{FragmentOutcome, RowOutcome};
use census_domain::model::ContactClaimEvidence;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub fn audit_table(outcomes: &[FragmentOutcome]) -> String {
    let mut table = String::from(
        "| fragment | rows | verified | role conveyed by page context | role contradicted | attribution-required | render-required | mismatch | empty | fetch failed | shipped share |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    let mut totals: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut rows_total = 0usize;
    for outcome in outcomes {
        let count = |verdict: super::verdict::Verdict| {
            outcome
                .counts
                .get(&verdict)
                .copied()
                .map_or(0, |value| value)
        };
        let total = outcome.rows.len();
        let shipped = count(super::verdict::Verdict::Ok);
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
            count(super::verdict::Verdict::AttributionRequired),
            count(super::verdict::Verdict::RenderRequired),
            count(super::verdict::Verdict::Mismatch),
            count(super::verdict::Verdict::Empty),
            count(super::verdict::Verdict::FetchFailed),
            share
        ));
    }
    let shipped_total = totals.get("ok").map_or(0, |value| *value);
    let share = pct(shipped_total, rows_total);
    table.push_str(&format!(
        "| **total ({})** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{:.1} %** |\n",
        outcomes.len(),
        rows_total,
        totals.get("ok").copied().map_or(0, |value| value),
        totals.get("ok_role_context").copied().map_or(0, |value| value),
        totals.get("role_contradicted").copied().map_or(0, |value| value),
        totals
            .get("attribution_required")
            .copied()
            .map_or(0, |value| value),
        totals.get("render_required").copied().map_or(0, |value| value),
        totals.get("mismatch").copied().map_or(0, |value| value),
        totals.get("empty").copied().map_or(0, |value| value),
        totals.get("fetch_failed").copied().map_or(0, |value| value),
        share
    ));
    table
}

fn pct(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        let numerator = u32::try_from(num).map_or(f64::MAX, f64::from);
        let denominator = u32::try_from(den).map_or(f64::MAX, f64::from);
        numerator * 100.0 / denominator
    }
}

pub fn write_audit_csv(path: &Path, outcomes: &[FragmentOutcome]) -> anyhow::Result<()> {
    census_store::read::publish_atomically(path, |temporary| {
        write_audit_csv_body(temporary, path, outcomes)
    })?;
    Ok(())
}

fn write_audit_csv_body(
    temporary: &Path,
    published: &Path,
    outcomes: &[FragmentOutcome],
) -> census_store::StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .from_path(temporary)
        .map_err(|error| census_store::read::csv_failure(published, error))?;
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
            "field_relationship_evidence",
        ])
        .map_err(|error| census_store::read::csv_failure(published, error))?;
    for outcome in outcomes {
        for row in &outcome.rows {
            let evidence = serde_json::to_string(&row.evidence).map_err(|error| {
                census_store::StoreError::Invariant {
                    detail: error.to_string(),
                }
            })?;
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
                    evidence.as_str(),
                ])
                .map_err(|error| census_store::read::csv_failure(published, error))?;
        }
    }
    writer
        .flush()
        .map_err(|source| census_store::StoreError::Io {
            path: published.to_path_buf(),
            source,
        })
}

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
        let claims: Vec<&ContactClaimEvidence> =
            rows.iter().flat_map(|row| &row.evidence).collect();
        census_store::read::write_snapshot_rows(&evidence_path(&path), &claims)?;
        census_store::read::publish_atomically(&path, |temporary| {
            write_state_file_body(temporary, &path, &rows)
        })?;
        counts.insert(state, rows.len());
    }
    Ok(counts)
}

fn write_state_file_body(
    temporary: &Path,
    published: &Path,
    rows: &[&RowOutcome],
) -> census_store::StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .from_path(temporary)
        .map_err(|error| census_store::read::csv_failure(published, error))?;
    writer
        .write_record(
            census_domain::model::CONTACT_COLUMNS
                .iter()
                .copied()
                .chain(std::iter::once(census_domain::model::CONTACT_PROOF_COLUMN)),
        )
        .map_err(|error| census_store::read::csv_failure(published, error))?;
    for row in rows {
        let proof = census_domain::model::compute_contact_proof(&row.row, &row.evidence).map_err(
            |source| census_store::StoreError::Invariant {
                detail: format!("contact proof failed before publication: {source}"),
            },
        )?;
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
                proof.as_str(),
            ])
            .map_err(|error| census_store::read::csv_failure(published, error))?;
    }
    writer
        .flush()
        .map_err(|source| census_store::StoreError::Io {
            path: published.to_path_buf(),
            source,
        })
}

pub fn write_manifest(
    path: &Path,
    files: &[PathBuf],
    outcomes: &[FragmentOutcome],
    union_dir: Option<&Path>,
) -> anyhow::Result<()> {
    use anyhow::Context;
    use sha2::{Digest, Sha256};
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create manifest dir {parent:?}"))?;
    }
    let mut manifest = String::new();
    manifest.push_str(&census_crawl::net::now_iso8601());
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
    if let Some(union_dir) = union_dir {
        let mut output_files: Vec<PathBuf> = std::fs::read_dir(union_dir)
            .with_context(|| format!("read union dir {union_dir:?}"))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .collect();
        output_files.sort();
        for path in &output_files {
            let bytes = std::fs::read(path).with_context(|| format!("read union file {path:?}"))?;
            let digest = Sha256::digest(&bytes);
            manifest.push_str(&format!("{digest:x}  {}\n", path.display()));
        }
    }
    manifest.push('\n');
    for outcome in outcomes {
        manifest.push_str(&serde_json::to_string(&outcome.summary())?);
        manifest.push('\n');
    }
    std::fs::write(path, manifest).with_context(|| format!("write manifest {path:?}"))?;
    Ok(())
}

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
