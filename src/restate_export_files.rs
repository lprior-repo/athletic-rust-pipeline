use super::state::Prepared;
use crate::{model::MatchRecord, restate_types::RowOutput};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::Write,
    path::Path,
};

#[derive(Serialize, Deserialize)]
pub struct Issue {
    pub source_key: String,
    pub row_key: String,
    pub stage: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub attempt: u32,
}

pub struct RowResult {
    pub source_key: String,
    pub row_key: String,
    pub output: Option<RowOutput>,
    pub issue: Option<Issue>,
    pub failed: bool,
}

fn issue_key(issue: &Issue) -> String {
    crate::restate_types::row_key(
        &issue.row_key,
        &format!(
            "{}\0{}\0{}\0{}",
            issue.stage, issue.code, issue.attempt, issue.message
        ),
    )
}

pub fn write(directory: &Path, prepared: &Prepared, mut rows: Vec<RowResult>) -> Result<()> {
    let issues_path = directory.join("issues.jsonl");
    let mut issues = crate::jsonl::load(&issues_path, issue_key)?
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let mut records = HashMap::<String, MatchRecord>::new();
    rows.sort_by(|left, right| left.source_key.cmp(&right.source_key));
    let temporary = directory.join("restate-results.jsonl.tmp");
    let mut results = File::create(&temporary)?;
    rows.into_iter().try_for_each(|row| -> Result<()> {
        if let Some(issue) = row.issue {
            issues.insert(issue_key(&issue), issue);
        }
        if let Some(output) = row.output {
            output.issues.iter().for_each(|issue| {
                let issue = Issue {
                    source_key: row.source_key.clone(),
                    row_key: row.row_key.clone(),
                    stage: issue.stage.clone(),
                    code: issue.code.clone(),
                    message: issue.message.clone(),
                    retryable: issue.retryable,
                    attempt: issue.attempt,
                };
                issues.insert(issue_key(&issue), issue);
            });
            serde_json::to_writer(&mut results, &output)?;
            results.write_all(b"\n")?;
            if records.insert(row.source_key, output.record).is_some() {
                anyhow::bail!("duplicate row in Restate projection");
            }
        }
        Ok(())
    })?;
    results.sync_all()?;
    fs::rename(temporary, directory.join("restate-results.jsonl"))?;
    atomic_lines(&issues_path, issues.values())?;
    let ordered = prepared
        .prospects
        .iter()
        .filter_map(|prospect| records.get(&prospect.source_key).cloned())
        .collect::<Vec<_>>();
    crate::output::write_all(directory, &ordered)?;
    atomic_lines(&directory.join("checkpoint.jsonl"), ordered.iter())?;
    crate::coverage::write_coverage(
        directory,
        &prepared.fingerprint,
        &prepared.prospects,
        &records,
    )?;
    File::open(directory)?.sync_all()?;
    Ok(())
}

fn atomic_lines<'a, T: Serialize + 'a>(
    path: &Path,
    mut values: impl Iterator<Item = &'a T>,
) -> Result<()> {
    let temporary = path.with_extension("jsonl.tmp");
    let mut file = File::create(&temporary)?;
    values.try_for_each(|value| -> Result<()> {
        serde_json::to_writer(&mut file, value)?;
        file.write_all(b"\n")?;
        Ok(())
    })?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    File::open(path.parent().context("projection has no parent")?)?.sync_all()?;
    Ok(())
}
