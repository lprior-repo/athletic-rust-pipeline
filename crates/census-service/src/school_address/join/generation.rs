use std::io::Write;
use std::path::Path;

use census_domain::school_directory::SchoolDirectoryEntry;
use census_store::Store;
use serde::Serialize;

use super::lanes::build_lane_evidence;
use super::{process, Counters, JoinError, JoinReport, LaneSet, Mode, OutcomeRow, Overrides};
use crate::school_address::{verify_current, Report, VerifiedGeneration};

#[derive(Serialize)]
struct ReportFile<'a> {
    mode: &'static str,
    generation: String,
    manifest_digest: &'a str,
    now: &'a Option<String>,
    lanes: &'a LaneSet,
    counters: &'a Counters,
}

pub fn join_generation(
    store: &Store,
    generation_dir: &Path,
    out_dir: Option<&Path>,
    overrides: Overrides,
    mode: Mode,
) -> Result<JoinReport, JoinError> {
    let overrides = overrides.validated()?;
    let generation = verify_current(generation_dir)?;
    let entries: Vec<SchoolDirectoryEntry> = parse_artifact(&generation, "school_directory.json")?;
    let report: Report = parse_artifact(&generation, "pipeline_report.json")?;
    let lanes = build_lane_evidence(&report, &overrides)?;
    let index = census_domain::school_directory::DirectoryIndex::build(&entries);
    let (counters, outcomes) = process(store, &index, &lanes, mode)?;

    let out_dir = match out_dir {
        Some(dir) => dir.to_path_buf(),
        None => store.root().join("out/school-address-join"),
    };
    std::fs::create_dir_all(&out_dir).map_err(|source| JoinError::Io {
        path: out_dir.clone(),
        source,
    })?;
    let report_path = out_dir.join("report.json");
    let outcomes_path = out_dir.join("outcomes.jsonl");
    let file = ReportFile {
        mode: mode.as_str(),
        generation: generation_dir.display().to_string(),
        manifest_digest: &report.manifest_digest,
        now: &report.now,
        lanes: &lanes,
        counters: &counters,
    };
    write_json(&report_path, &file)?;
    write_outcomes(&outcomes_path, &outcomes)?;

    Ok(JoinReport {
        mode: mode.as_str().to_string(),
        generation: generation_dir.display().to_string(),
        report: report_path.display().to_string(),
        outcomes: outcomes_path.display().to_string(),
        counters,
        lanes,
    })
}

fn parse_artifact<T: serde::de::DeserializeOwned>(
    generation: &VerifiedGeneration,
    name: &'static str,
) -> Result<T, JoinError> {
    let bytes = generation.artifact(name)?;
    serde_json::from_slice(bytes).map_err(|source| JoinError::Artifact { name, source })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), JoinError> {
    let encoded = serde_json::to_vec_pretty(value).map_err(|source| JoinError::Invariant {
        detail: format!("the join report is not valid json: {source}"),
    })?;
    std::fs::write(path, encoded).map_err(|source| JoinError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_outcomes(path: &Path, rows: &[OutcomeRow]) -> Result<(), JoinError> {
    let file = std::fs::File::create(path).map_err(|source| JoinError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut writer = std::io::BufWriter::new(file);
    for row in rows {
        serde_json::to_writer(&mut writer, row).map_err(|source| JoinError::Invariant {
            detail: format!("an outcome row is not valid json: {source}"),
        })?;
        writer.write_all(b"\n").map_err(|source| JoinError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    writer.flush().map_err(|source| JoinError::Io {
        path: path.to_path_buf(),
        source,
    })
}
