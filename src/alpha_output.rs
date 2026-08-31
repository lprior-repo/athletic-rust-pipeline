use crate::alpha_checkpoint::ensure_exists;
use crate::alpha_normalize::{ResultRecord, SourceAthlete};
use crate::alpha_output_privacy::validate_value;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
const ATHLETE_HEADERS: &[&str] = &[
    "athlete_id",
    "athlete_name",
    "school",
    "state",
    "graduation_year",
    "cohort_evidence",
    "gender",
    "sport",
    "profile_url",
    "source_urls_json",
    "result_count",
];
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortException {
    pub athlete_id: u64,
    pub reason: String,
    pub source_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnresolvedRecord {
    pub record_key: String,
    pub reason: String,
    pub source_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CoverageReport {
    pub planned_units: usize,
    pub complete_units: usize,
    pub empty_units: usize,
    pub incomplete_units: usize,
    pub exception_units: usize,
}
#[derive(Serialize)]
struct ResultLine<'a> {
    athlete_id: u64,
    #[serde(flatten)]
    result: &'a ResultRecord,
}
#[allow(dead_code)]
pub fn approved_athlete_headers() -> &'static [&'static str] {
    ATHLETE_HEADERS
}

pub fn validate_public_json(value: &Value) -> Result<()> {
    validate_value(value, "output")
}

pub fn write_outputs(
    output_dir: &Path,
    athletes: &[SourceAthlete],
    cohort_exceptions: &[CohortException],
    unresolved: &[UnresolvedRecord],
    coverage: &CoverageReport,
) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating output directory {}", output_dir.display()))?;
    ensure_exists(output_dir)?;
    write_athletes_csv(&output_dir.join("athletes.csv"), athletes)?;
    write_jsonl(&output_dir.join("athletes.jsonl"), athletes)?;
    let results: Vec<ResultLine<'_>> = athletes
        .iter()
        .flat_map(|athlete| {
            athlete.results.iter().map(move |result| ResultLine {
                athlete_id: athlete.athlete_id,
                result,
            })
        })
        .collect();
    write_jsonl(&output_dir.join("results.jsonl"), &results)?;
    write_jsonl(&output_dir.join("cohort-exceptions.jsonl"), cohort_exceptions)?;
    write_csv_records(&output_dir.join("unresolved.csv"), unresolved)?;
    write_jsonl(&output_dir.join("unresolved.jsonl"), unresolved)?;
    write_json(&output_dir.join("coverage.json"), coverage)?;
    Ok(())
}
pub fn write_athletes_csv(path: &Path, athletes: &[SourceAthlete]) -> Result<()> {
    let mut output = String::new();
    output.push_str(&ATHLETE_HEADERS.join(","));
    output.push('\n');
    for athlete in athletes {
        let source_urls_json =
            serde_json::to_string(&athlete.source_urls).context("serializing source URLs")?;
        let profile_url = match (athlete.profile_url.is_empty(), athlete.profile_urls.first()) {
            (false, _) => athlete.profile_url.clone(),
            (true, Some(url)) => url.clone(),
            (true, None) => String::new(),
        };
        let graduation_year = athlete
            .graduation_year
            .map_or_else(String::new, |year| year.to_string());
        let athlete_name = if athlete.athlete_name.is_empty() {
            format!("{} {}", athlete.first_name, athlete.last_name).trim().to_owned()
        } else {
            athlete.athlete_name.clone()
        };
        let fields = [
            athlete.athlete_id.to_string(),
            athlete_name,
            athlete.school.clone(),
            athlete.state.clone(),
            graduation_year,
            athlete.cohort_evidence.clone(),
            athlete.gender.clone(),
            athlete.sport.clone(),
            profile_url,
            source_urls_json,
            athlete.results.len().to_string(),
        ];
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            if index != 0 && index != 10 {
                validate_public_json(&Value::String(field.clone()))?;
            }
            output.push_str(&csv_escape(field));
        }
        output.push('\n');
    }
    write_atomic(path, output.as_bytes())
}

fn write_csv_records(path: &Path, records: &[UnresolvedRecord]) -> Result<()> {
    let mut output = String::from("record_key,reason,source_url\n");
    for record in records {
        for (index, field) in [&record.record_key, &record.reason, &record.source_url]
            .iter()
            .enumerate()
        {
            if index > 0 {
                output.push(',');
            }
            let value = Value::String((*field).clone());
            validate_public_json(&value)?;
            output.push_str(&csv_escape(field));
        }
        output.push('\n');
    }
    write_atomic(path, output.as_bytes())
}

fn write_jsonl<T: Serialize>(path: &Path, records: &[T]) -> Result<()> {
    let mut output = Vec::new();
    for record in records {
        let value = serde_json::to_value(record).context("serializing JSONL record")?;
        validate_public_json(&value)?;
        serde_json::to_writer(&mut output, &value).context("encoding JSONL record")?;
        output.push(b'\n');
    }
    write_atomic(path, &output)
}
pub fn read_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter(|(_, line)| line.as_ref().map_or(true, |value| !value.trim().is_empty()))
        .map(|(line_number, line)| {
            let line = line.with_context(|| format!("reading JSONL line {}", line_number + 1))?;
            serde_json::from_str(&line)
                .with_context(|| format!("decoding JSONL line {}", line_number + 1))
        })
        .collect()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let value = serde_json::to_value(value).context("serializing JSON output")?;
    validate_public_json(&value)?;
    let output = serde_json::to_vec_pretty(&value).context("encoding JSON output")?;
    write_atomic(path, &output)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary)
        .with_context(|| format!("creating temporary output {}", temporary.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("writing temporary output {}", temporary.display()))?;
    file.sync_all().context("syncing temporary output")?;
    fs::rename(&temporary, path)
        .with_context(|| format!("renaming {}", temporary.display()))
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha_normalize::SourceAthlete;

    #[test]
    fn approved_headers_are_stable() {
        assert_eq!(approved_athlete_headers()[0], "athlete_id");
        assert_eq!(approved_athlete_headers().last(), Some(&"result_count"));
    }

    #[test]
    fn privacy_guard_rejects_forbidden_key_and_values() {
        assert!(validate_public_json(&serde_json::json!({"email": "x@y.example"})).is_err());
        assert!(validate_public_json(&serde_json::json!({"note": "Bearer token"})).is_err());
        assert!(validate_public_json(&serde_json::json!({"note": "555-123-4567"})).is_err());
    }

    #[test]
    fn output_writes_required_files() {
        let directory = tempfile::tempdir().expect("tempdir");
        let athlete = SourceAthlete {
            athlete_id: 7,
            first_name: "Test".to_owned(),
            last_name: "Runner".to_owned(),
            profile_urls: vec!["https://athletic.net/athlete/7".to_owned()],
            ..Default::default()
        };
        write_outputs(
            directory.path(),
            &[athlete],
            &[],
            &[],
            &CoverageReport {
                planned_units: 1,
                complete_units: 1,
                ..Default::default()
            },
        )
        .expect("write outputs");
        for name in [
            "athletes.csv",
            "athletes.jsonl",
            "results.jsonl",
            "cohort-exceptions.jsonl",
            "unresolved.csv",
            "coverage.json",
        ] {
            assert!(directory.path().join(name).is_file(), "missing {name}");
        }
        let csv = std::fs::read_to_string(directory.path().join("athletes.csv")).expect("csv");
        assert_eq!(csv.lines().next(), Some(ATHLETE_HEADERS.join(",").as_str()));
    }
}
