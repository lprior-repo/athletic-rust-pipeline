use super::super::*;
use super::fixtures::Course;
use serde_json::{json, Value};

fn rows(fixture: &Fixture) -> TestResult<(::csv::StringRecord, Vec<::csv::StringRecord>)> {
    let data = crate::export::ExportDataset::load(&fixture.store)?;
    let selected = bests::build_from_dataset(
        &data,
        &bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    );
    let (_, path) = bests::write(fixture.dir.path(), &selected, "xc-context-fixture")?;
    let mut reader = ::csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    let records = reader.records().collect::<Result<_, _>>()?;
    Ok((header, records))
}

fn cell<'a>(
    header: &::csv::StringRecord,
    row: &'a ::csv::StringRecord,
    name: &str,
) -> TestResult<&'a str> {
    let column = header
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("missing {name}"))?;
    row.get(column)
        .ok_or_else(|| format!("missing {name} value").into())
}

fn specification(course: Option<&str>, measurement: &str, conditions: &str) -> Value {
    json!({
        "cross_country": {
            "distance": { "unit": "metres", "thousandths": 5_000_000 },
            "course": course.map(|id| [id, "1"]),
            "measurement": measurement, "conditions": conditions,
        },
        "hurdles": null, "implement": null, "indoor_track": null, "category": "boys",
    })
}

pub(super) fn courses(fixture: &Fixture, courses: &[Course]) -> TestResult {
    let (header, rows) = rows(fixture)?;
    let mut retained = std::collections::BTreeSet::new();
    for row in &rows {
        if cell(&header, row, "event")? != "CrossCountry" {
            continue;
        }
        check!(eq; cell(&header, row, "comparison_policy")?, "same_course");
        let source: Value = serde_json::from_str(cell(&header, row, "source_specification")?)?;
        let comparison: Value =
            serde_json::from_str(cell(&header, row, "comparison_specification")?)?;
        let id = source
            .pointer("/cross_country/course/0")
            .and_then(Value::as_str)
            .ok_or("missing course ID")?;
        let course = courses
            .iter()
            .find(|course| course.id == id)
            .ok_or("unexpected course")?;
        let expected = specification(Some(&course.id), "published_short", &course.conditions);
        check!(eq; source, expected);
        check!(eq; comparison, expected);
        check!(eq; cell(&header, row, "best_mark")?, course.mark);
        check!(eq; cell(&header, row, "athlete_id")?, fixture.julian.as_str());
        check!(eq; cell(&header, row, "date")?, "2026-09-15");
        check!(retained.insert(id.to_owned()), "duplicate course {id}");
    }
    let expected: std::collections::BTreeSet<_> =
        courses.iter().map(|course| course.id.clone()).collect();
    check!(eq; retained, expected);
    Ok(())
}

pub(super) fn five_k(fixture: &Fixture, mark: &str) -> TestResult {
    let (header, rows) = rows(fixture)?;
    let mut policies = std::collections::BTreeSet::new();
    for row in &rows {
        if cell(&header, row, "event")? != "CrossCountry" {
            continue;
        }
        let policy = cell(&header, row, "comparison_policy")?;
        let course = match policy {
            "observed_fastest" => None,
            "same_course" => Some("fixture-five-k"),
            _ => return Err("unexpected XC comparison policy".into()),
        };
        let source: Value = serde_json::from_str(cell(&header, row, "source_specification")?)?;
        let comparison: Value =
            serde_json::from_str(cell(&header, row, "comparison_specification")?)?;
        check!(eq; source, specification(Some("fixture-five-k"), "published_measured", "dry"));
        check!(eq; comparison, specification(course, "published_measured", "dry"));
        check!(eq; cell(&header, row, "best_mark")?, mark);
        check!(eq; cell(&header, row, "athlete_id")?, fixture.julian.as_str());
        check!(
            policies.insert(policy.to_owned()),
            "duplicate policy {policy}"
        );
    }
    check!(eq; policies, std::collections::BTreeSet::from(["observed_fastest".to_owned(), "same_course".to_owned()]));
    Ok(())
}
