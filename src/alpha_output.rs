use crate::alpha_normalize::{ResultRecord, SourceAthlete};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
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
const FORBIDDEN_KEYS: &[&str] = &[
    "email",
    "phone",
    "street",
    "postal",
    "cookie",
    "authorization",
    "token",
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
        let profile_url = athlete.profile_url.clone();
        let graduation_year = athlete
            .graduation_year
            .map_or_else(String::new, |year| year.to_string());
        let fields = [
            athlete.athlete_id.to_string(),
            athlete.athlete_name.clone(),
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

fn validate_value(value: &Value, path: &str) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let lower = key.to_ascii_lowercase();
                if FORBIDDEN_KEYS.iter().any(|forbidden| lower.contains(forbidden)) {
                    bail!("forbidden output field at {path}.{key}");
                }
                validate_value(child, &format!("{path}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                validate_value(child, &format!("{path}[{index}]"))?;
            }
        }
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            if lower.contains("cookie") || lower.contains("authorization") || lower.contains("token") {
                bail!("forbidden sensitive value at {path}");
            }
            if looks_like_email(text) || looks_like_phone(text) {
                bail!("forbidden personal value at {path}");
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn looks_like_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.trim().is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn looks_like_phone(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() == 10 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    let digits = trimmed.chars().filter(|c| c.is_ascii_digit()).count();
    let separators = trimmed.chars().filter(|c| matches!(c, ' ' | '-' | '(' | ')' | '.')).count();
    digits >= 7 && separators >= 2 && !is_iso_date(trimmed)
}

fn is_iso_date(text: &str) -> bool {
    text.len() == 10
        && text.as_bytes().get(4) == Some(&b'-')
        && text.as_bytes().get(7) == Some(&b'-')
        && text.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
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
