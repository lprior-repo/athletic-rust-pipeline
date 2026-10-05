mod judge;
mod load;
mod patterns;
mod report;
use anyhow::{bail, Context, Result};
use census_domain::model::{
    ContactClaimEvidence, RawContactRow, CONTACT_COLUMNS, CONTACT_PROOF_COLUMN,
};
use clap::Args;
use load::load_rows;
use report::print_report;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct Row {
    school: String,
    city: String,
    state: String,
    sport: String,
    role: String,
    coach_name: String,
    public_professional_email: String,
    ad_name: String,
    ad_email: String,
    source_url: String,
    last_observed: String,
    verified_proof_digest: String,
}

impl Row {
    pub(super) fn from_fields(fields: Vec<String>) -> Self {
        load::from_fields(fields)
    }
    pub(super) fn to_fields(&self) -> Vec<String> {
        load::to_fields(self)
    }
    pub(super) fn dedupe_key(&self) -> (String, String, String, String) {
        load::dedupe_key(self)
    }
    pub(super) fn normalize(&mut self) {
        load::normalize(self)
    }
    pub(super) fn judge(&self, state: &str) -> Option<String> {
        judge::judge(self, state)
    }
}

#[derive(Debug, Args)]
pub(super) struct MergeCoachesArgs {
    #[arg(long, value_name = "DIR")]
    pub(super) fragments: PathBuf,
    #[arg(long, value_name = "CSV")]
    pub(super) out: PathBuf,
    #[arg(long, value_name = "MD")]
    pub(super) report: PathBuf,
}

#[derive(Debug, Default)]
struct StateCounts {
    rows: usize,
    kept: usize,
    duplicates: usize,
    rejected: usize,
}

#[derive(Debug, Clone)]
struct Rejection {
    state: String,
    line_no: usize,
    reason: String,
    school: String,
    coach_name: String,
    role: String,
    source_url: String,
}

pub(super) fn run_merge_coaches(args: &MergeCoachesArgs) -> Result<()> {
    if !args.fragments.is_dir() {
        bail!("no fragment directory {}", args.fragments.display());
    }
    let csv_files = collect_csv_files(&args.fragments)?;
    let pass = process_files(&csv_files)?;
    let evidence = collect_merged_evidence(&pass.kept, &csv_files)?;
    write_merged_csv(&pass.kept, args)?;
    census_store::read::write_snapshot_rows(
        &census_service::coachverify::evidence_path(&args.out),
        &evidence,
    )?;
    print_report(
        &pass.kept,
        &pass.per_state,
        &pass.rejects,
        &pass.broken,
        &args.out,
        &args.report,
    );
    Ok(())
}

#[derive(Default)]
struct MergePass {
    kept: KeptRows,
    per_state: BTreeMap<String, StateCounts>,
    rejects: Vec<Rejection>,
    broken: Vec<(String, String)>,
}

fn process_files(csv_files: &[PathBuf]) -> Result<MergePass> {
    let mut pass = MergePass::default();
    for path in csv_files {
        let state = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map_or(Default::default(), core::convert::identity)
            .to_uppercase();
        let (rows, problem) = load_rows(path, &state);
        if let Some(problem) = problem {
            pass.broken.push((
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .map_or(Default::default(), core::convert::identity),
                problem,
            ));
            continue;
        }
        for (line_no, mut row) in rows {
            let counts = pass.per_state.entry(state.clone()).or_default();
            counts.rows = counts.rows.saturating_add(1);
            row.normalize();
            if let Some(reason) = Row::judge(&row, &state) {
                counts.rejected = counts.rejected.saturating_add(1);
                pass.rejects.push(Rejection {
                    state: state.clone(),
                    line_no,
                    reason,
                    school: row.school.clone(),
                    coach_name: row.coach_name.clone(),
                    role: row.role.clone(),
                    source_url: row.source_url.clone(),
                });
                continue;
            }
            let key = row.dedupe_key();
            if !pass.kept.contains_key(&key) {
                pass.kept.insert(key, row);
                counts.kept = counts.kept.saturating_add(1);
            } else {
                counts.duplicates = counts.duplicates.saturating_add(1);
                if let Some(existing) = pass.kept.get(&key) {
                    if pick_richer(&row, existing) {
                        pass.kept.insert(key, row);
                    }
                }
            }
        }
    }
    Ok(pass)
}

type RowKey = (String, String, String, String);

type KeptRows = BTreeMap<RowKey, Row>;

fn collect_csv_files(frag_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut csv_files: Vec<PathBuf> = Vec::new();
    for entry in frag_dir
        .read_dir()
        .with_context(|| "reading fragment directory")?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "csv") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.len() == 2 && stem.is_ascii() {
                    csv_files.push(path);
                }
            }
        }
    }
    csv_files.sort();
    Ok(csv_files)
}

fn pick_richer(new: &Row, existing: &Row) -> bool {
    let new_has_email = !new.public_professional_email.trim().is_empty();
    let old_has_email = !existing.public_professional_email.trim().is_empty();
    (new_has_email && !old_has_email) || new.last_observed.trim() > existing.last_observed.trim()
}

fn write_merged_csv(
    kept: &BTreeMap<(String, String, String, String), Row>,
    args: &MergeCoachesArgs,
) -> Result<()> {
    let published = &args.out;
    census_store::read::publish_atomically(published, |temporary| {
        write_merged_body(temporary, published, kept)
    })?;
    Ok(())
}

fn write_merged_body(
    temporary: &std::path::Path,
    published: &std::path::Path,
    kept: &BTreeMap<(String, String, String, String), Row>,
) -> census_store::StoreResult<()> {
    let mut writer = csv::Writer::from_path(temporary)
        .map_err(|error| census_store::read::csv_failure(published, error))?;
    writer
        .write_record(
            CONTACT_COLUMNS
                .iter()
                .copied()
                .chain(std::iter::once(CONTACT_PROOF_COLUMN)),
        )
        .map_err(|error| census_store::read::csv_failure(published, error))?;
    for row in kept.values() {
        writer
            .write_record(row.to_fields())
            .map_err(|error| census_store::read::csv_failure(published, error))?;
    }
    writer
        .flush()
        .map_err(|source| census_store::StoreError::Io {
            path: published.to_path_buf(),
            source,
        })
}

fn collect_merged_evidence(
    kept: &KeptRows,
    csv_files: &[PathBuf],
) -> Result<Vec<ContactClaimEvidence>> {
    let wanted: Vec<RawContactRow> = kept.values().map(raw_contact_row).collect();
    let mut published: Vec<ContactClaimEvidence> = Vec::new();
    for path in csv_files {
        let sidecar = census_service::coachverify::evidence_path(path);
        if !sidecar.is_file() {
            bail!(
                "fragment {} has no evidence sidecar {}",
                path.display(),
                sidecar.display()
            );
        }
        let claims = census_service::coachverify::read_evidence_jsonl(&sidecar)?;
        for row in &wanted {
            published.extend(census_service::coachverify::claims_for_row(&claims, row));
        }
    }
    Ok(published)
}

fn raw_contact_row(row: &Row) -> RawContactRow {
    RawContactRow {
        school: row.school.clone(),
        city: row.city.clone(),
        state: row.state.clone(),
        sport: row.sport.clone(),
        role: row.role.clone(),
        coach_name: row.coach_name.clone(),
        public_professional_email: row.public_professional_email.clone(),
        ad_name: row.ad_name.clone(),
        ad_email: row.ad_email.clone(),
        source_urls: row
            .source_url
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        last_observed: row.last_observed.clone(),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
