use super::*;
use crate::report::ReportError;
use serde_json::Value;
use std::path::PathBuf;

mod manifest;

const EARLIER_ID: &str = "perf_7eecb436751a53a7";
const LATER_ID: &str = "perf_f65b501d90073204";
const SMALLER_ID: &str = "perf_57223508d52514de";
const LARGER_ID: &str = "perf_818b4865f6c57843";

#[derive(Clone, Copy)]
enum Sidecar {
    Csv,
    Jsonl,
}

impl Sidecar {
    fn name(self) -> &'static str {
        match self {
            Self::Csv => "best-results-co2027.csv",
            Self::Jsonl => "best-results-co2027.jsonl",
        }
    }

    fn replace(self, path: &Path, wrong: &Observation) -> TestResult<Vec<u8>> {
        match self {
            Self::Csv => forge_csv(path, wrong),
            Self::Jsonl => forge_jsonl(path, wrong),
        }
    }

    fn assert_semantic_refusal(self, path: &Path, changed_field: &str) -> TestResult {
        let detail = match crate::workbook::publication::verify_published(path) {
            Err(ReportError::Invariant { detail }) => detail,
            Err(error) => return Err(format!("unexpected forgery refusal: {error}").into()),
            Ok(_) => return Err("forged equal-mark representative was verified".into()),
        };
        check!(detail.contains(self.name()), "{detail}");
        check!(!detail.contains("generation artifact mismatch"), "{detail}");
        check!(!detail.contains("corrupt generation manifest"), "{detail}");
        match self {
            Self::Csv => {
                let column = match changed_field {
                    "date" => "column 16:",
                    "performance_id" => "column 23:",
                    _ => return Err("unsupported semantic forgery field".into()),
                };
                check!(detail.contains("record 2"), "{detail}");
                check!(detail.contains(column), "{detail}");
            }
            Self::Jsonl => {
                check!(
                    detail.contains("line 1 does not match the shared best-results reduction"),
                    "{detail}"
                );
                check!(detail.contains(changed_field), "{detail}");
            }
        }
        Ok(())
    }
}

fn forge_csv(path: &Path, wrong: &Observation) -> TestResult<Vec<u8>> {
    let (headers, original) = artifacts::csv_row(path)?;
    let mut cells = original.iter().map(str::to_string).collect::<Vec<_>>();
    [
        ("date", wrong.performance.date.as_str()),
        ("meet", wrong.meet.name.as_str()),
        ("performance_id", wrong.performance.id.as_str()),
        ("source_key", wrong.performance.source_key.as_str()),
    ]
    .into_iter()
    .try_for_each(|(header, value)| -> TestResult {
        let column = headers
            .iter()
            .position(|name| name == header)
            .ok_or_else(|| format!("missing forgery CSV column {header}"))?;
        let cell = cells.get_mut(column).ok_or("forgery CSV cell missing")?;
        *cell = value.into();
        Ok(())
    })?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(&headers)?;
    writer.write_record(&cells)?;
    writer.flush()?;
    Ok(writer.into_inner()?)
}

fn forge_jsonl(path: &Path, wrong: &Observation) -> TestResult<Vec<u8>> {
    let mut found = artifacts::jsonl_row(path)?;
    [
        ("date", wrong.performance.date.as_str()),
        ("meet", wrong.meet.name.as_str()),
        ("meet_id", wrong.performance.meet.as_str()),
        ("performance_id", wrong.performance.id.as_str()),
        ("source_key", wrong.performance.source_key.as_str()),
        ("result_url", artifacts::result_url(wrong)?),
    ]
    .into_iter()
    .try_for_each(|(field, value)| -> TestResult {
        let cell = found.get_mut(field).ok_or("forgery JSONL field missing")?;
        *cell = Value::from(value);
        Ok(())
    })?;
    let mut bytes = serde_json::to_vec(&found)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn publish_fixture(
    fixture: &TieFixture,
    observations: &[Observation; 2],
    expected: &Observation,
    directory: &Path,
) -> TestResult<PathBuf> {
    let store = fixture.store_with_replay(&directory.join("store"), observations)?;
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: Some(directory.join("publication")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("forgery fixture season")?,
    };
    let path = crate::workbook::build(&store, &options)?;
    let generation = path.parent().ok_or("forgery generation directory")?;
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    let row = assert_winner(&dataset, expected)?;
    check!(eq; row.result.value, 50_000_000_000);
    artifacts::assert_bests(generation, expected, &row)?;
    drop(dataset);
    crate::workbook::publication::verify_published(&path)?;
    Ok(path)
}

fn assert_forgeries(
    fixture: &TieFixture,
    observations: &[Observation; 2],
    expected: &Observation,
    wrong: &Observation,
    changed_field: &str,
) -> TestResult {
    check!(eq; expected.performance.mark, wrong.performance.mark);
    check!(eq; expected.performance.athlete, wrong.performance.athlete);
    check!(eq; expected.performance.source_athlete, wrong.performance.source_athlete);
    let directory = tempfile::tempdir()?;
    let path = publish_fixture(fixture, observations, expected, directory.path())?;
    let original = path.parent().ok_or("original forgery generation")?;
    let frozen = std::fs::read(original.join("frozen-input.json"))?;
    [Sidecar::Csv, Sidecar::Jsonl]
        .into_iter()
        .try_for_each(|sidecar| -> TestResult {
            let artifact = original.join(sidecar.name());
            let unchanged = std::fs::read(&artifact)?;
            let valid = manifest::install(original, &directory.path().join("unchanged"), sidecar.name(), &unchanged)?;
            let verified = crate::workbook::publication::verify_published(&valid)?;
            check!(eq; valid.parent().and_then(Path::file_name).and_then(|name| name.to_str()), Some(verified.generation_digest.as_str()));
            check!(eq; verified.workbook.mapped_athletes, 1);
            let replacement = sidecar.replace(&artifact, wrong)?;
            check!(ne; replacement, unchanged);
            let forged = manifest::install(original, &directory.path().join("forged"), sidecar.name(), &replacement)?;
            check!(eq; std::fs::read(forged.parent().ok_or("forged generation")?.join("frozen-input.json"))?, frozen);
            sidecar.assert_semantic_refusal(&forged, changed_field)
        })?;
    check!(eq; std::fs::read(original.join("frozen-input.json"))?, frozen);
    crate::workbook::publication::verify_published(&path)?;
    Ok(())
}

#[test]
fn semantic_verification_refuses_a_later_date_equal_mark_representative_in_both_sidecars(
) -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("forgery time fixture")?;
    let earlier = fixture.report(case, "artifact-earlier", "2024-02-29")?;
    let later = fixture.report(case, "artifact-later", "2026-04-01")?;
    check!(eq; earlier.performance.id.as_str(), EARLIER_ID);
    check!(eq; later.performance.id.as_str(), LATER_ID);
    assert_forgeries(
        &fixture,
        &[later.clone(), earlier.clone()],
        &earlier,
        &later,
        "date",
    )
}

#[test]
fn semantic_verification_refuses_a_larger_id_same_date_representative_in_both_sidecars(
) -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("forgery time fixture")?;
    let smaller = fixture.report(case, "artifact-smaller-id", "2026-04-01")?;
    let mut larger = fixture.report(case, "artifact-larger-id", "2026-04-01")?;
    check!(eq; smaller.performance.id.as_str(), SMALLER_ID);
    check!(eq; larger.performance.id.as_str(), LARGER_ID);
    check!(eq; smaller.performance.date, larger.performance.date);
    larger.meet = smaller.meet.clone();
    larger.event = smaller.event.clone();
    larger.performance.meet = smaller.performance.meet.clone();
    larger.performance.event = smaller.performance.event.clone();
    assert_forgeries(
        &fixture,
        &[larger.clone(), smaller.clone()],
        &smaller,
        &larger,
        "performance_id",
    )
}
