use super::*;
use census_domain::model::ExactSeconds;

fn published_rows(labels: &[(&str, &str)]) -> TestResult<Value> {
    let mut document = document()?;
    let template = document
        .get("data")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("owned template")?
        .clone();
    let rows = labels
        .iter()
        .enumerate()
        .map(|(index, (label, mark))| {
            let mut row = template.clone();
            row["id"] = json!(u64::try_from(index)?
                .checked_add(900_000)
                .ok_or("result id")?);
            row["eventName"] = json!(label);
            row["mark"] = json!(mark);
            row["units"] = Value::Null;
            Ok(row)
        })
        .collect::<TestResult<Vec<_>>>()?;
    document["data"] = json!(rows);
    Ok(document)
}

fn event_for<'a>(
    accumulated: &'a Accumulator,
    result: &str,
) -> TestResult<(
    &'a census_domain::model::CanonicalEvent,
    &'a census_domain::model::CanonicalPerformance,
)> {
    let performance = accumulated
        .performances
        .values()
        .find(|row| row.source_key == result)
        .ok_or("projected result")?;
    let event = accumulated
        .events
        .get(performance.event.as_str())
        .ok_or("event reference")?;
    Ok((event, performance))
}

#[test]
fn owned_same_meet_masses_separate_refs_without_splitting_equivalent_units() -> TestResult {
    let document = published_rows(&[
        ("Shot Put (4kg)", "40-0"),
        ("Shot Put (3kg)", "45-0"),
        ("Shot Put (4000g)", "42-0"),
        ("Shot Put", "50-0"),
    ])?;
    let (accumulated, _) = project(document, &[school("Spann", "38332")], metadata()?)?;
    let (four, _) = event_for(&accumulated, "milesplit_result:900000")?;
    let (three, _) = event_for(&accumulated, "milesplit_result:900001")?;
    let (equivalent, _) = event_for(&accumulated, "milesplit_result:900002")?;
    let (unknown, _) = event_for(&accumulated, "milesplit_result:900003")?;
    check!(eq; four.id, equivalent.id);
    check!(four.id != three.id);
    check!(four.id != unknown.id);
    check!(eq; four.specification.implement.ok_or("mass")?.micrograms(), 4_000_000_000);
    check!(eq; three.specification.implement.ok_or("mass")?.micrograms(), 3_000_000_000);
    check!(eq; unknown.specification.implement, None);
    check!(eq; four.resolved_specification()?, equivalent.resolved_specification()?);
    check!(four
        .source_labels
        .iter()
        .any(|row| row.label == "Shot Put (4000g)"));
    Ok(())
}

#[test]
fn owned_hurdle_dimensions_are_minted_before_exact_time_performance_references() -> TestResult {
    let document = published_rows(&[
        ("100 Meter Hurdles (30in)", "14.251"),
        ("100 Meter Hurdles (33in)", "14.249"),
        ("100 Meter Hurdles (76.2cm)", "14.251"),
    ])?;
    let (accumulated, _) = project(document, &[school("Spann", "38332")], metadata()?)?;
    let (low, performance) = event_for(&accumulated, "milesplit_result:900000")?;
    let (high, _) = event_for(&accumulated, "milesplit_result:900001")?;
    let (equivalent, _) = event_for(&accumulated, "milesplit_result:900002")?;
    check!(eq; low.id, equivalent.id);
    check!(low.id != high.id);
    check!(eq; low.specification.hurdles.ok_or("height")?.height_micrometres, 762_000);
    check!(eq; high.specification.hurdles.ok_or("height")?.height_micrometres, 838_200);
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("14.251")?));
    Ok(())
}
