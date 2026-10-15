use super::*;
use crate::bests::{
    public_identity_selection_for_test as selection, write_selected_identity_for_test as write,
};
use crate::export::current_school::fixture::capture;
use crate::export::public_identity::tests::fixture::{merge, source_births, Fixture, TestResult};
use crate::report::ReportError;
use census_domain::model::EventSemantics;
use census_store::Store;

#[test]
fn independent_selected_readback_accepts_real_merge_outputs_and_reopened_authority() -> TestResult {
    let fixture = source_births()?;
    merge(&fixture)?;
    let (view, _) = capture(&fixture.store)?;
    let [_, second, _] = &fixture.athletes;
    let rows = [selection(second)];
    let output = fixture.directory.path().join("publication");
    write(
        &output,
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &view,
    )?;
    verify(&output, "co2027", &rows, &view)?;
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (store, _) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let (reopened, _) = capture(&store)?;
    verify(&output, "co2027", &rows, &reopened)?;
    Ok(())
}

#[test]
fn independent_selected_readback_refuses_candidate_fallback_in_csv_primary_join_key() -> TestResult
{
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [first, _, _] = &fixture.athletes;
    let rows = [selection(first)];
    let output = fixture.directory.path().join("publication");
    let (_, csv) = write(
        &output,
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &view,
    )?;
    replace_csv(&csv, "athlete_id", first.id.as_str())?;
    let [birth, _] = &fixture.births;
    let expected = format!(
        "{} record 2 column 0: expected {:?}, found {:?}",
        csv.display(),
        birth.public_id().as_str(),
        first.id.as_str()
    );
    match verify(&output, "co2027", &rows, &view) {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("unexpected CSV refusal: {error}").into()),
        Ok(()) => return Err("candidate fallback passed independent readback".into()),
    }
    Ok(())
}

#[test]
fn independent_selected_readback_rejects_changed_public_candidate_resolution_schema_and_original_selection(
) -> TestResult {
    let fixture = source_births()?;
    merge(&fixture)?;
    let (view, _) = capture(&fixture.store)?;
    let [first, second, _] = &fixture.athletes;
    let rows = [selection(second)];
    let output = fixture.directory.path().join("publication");
    let (jsonl, _) = write(
        &output,
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &view,
    )?;
    let original: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&jsonl)?)?;
    for (field, replacement) in [
        ("athlete_id", serde_json::json!(second.id)),
        (
            "canonical_candidate_id",
            serde_json::json!("absent-candidate"),
        ),
        ("identity_resolution", serde_json::Value::Null),
        ("schema", serde_json::json!("selected_identity_v0")),
        ("selection", serde_json::to_value(selection(first))?),
    ] {
        let mut changed = original.clone();
        let cell = changed.get_mut(field).ok_or("expected selected field")?;
        *cell = replacement;
        write_json(&jsonl, &changed)?;
        mismatch(verify(&output, "co2027", &rows, &view), &jsonl, field)?;
    }
    let mut extra = original.clone();
    extra
        .as_object_mut()
        .ok_or("selected object")?
        .insert("unbound_identity".into(), serde_json::json!(first.id));
    write_json(&jsonl, &extra)?;
    mismatch(
        verify(&output, "co2027", &rows, &view),
        &jsonl,
        "unbound_identity",
    )?;
    write_json(
        &jsonl,
        &serde_json::to_value(rows.first().ok_or("selection")?)?,
    )?;
    mismatch(verify(&output, "co2027", &rows, &view), &jsonl, "")?;
    Ok(())
}

#[test]
fn independent_selected_readback_uses_held_identity_head_instead_of_current_tables() -> TestResult {
    let fixture = source_births()?;
    let (held, _) = capture(&fixture.store)?;
    let [_, second, _] = &fixture.athletes;
    let rows = [selection(second)];
    let output = fixture.directory.path().join("publication");
    let (_, csv) = write(
        &output,
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &held,
    )?;
    merge(&fixture)?;
    let (current, _) = capture(&fixture.store)?;
    verify(&output, "co2027", &rows, &held)?;
    let [first_birth, second_birth] = &fixture.births;
    let expected = format!(
        "{} record 2 column 0: expected {:?}, found {:?}",
        csv.display(),
        first_birth.public_id().as_str(),
        second_birth.public_id().as_str()
    );
    match verify(&output, "co2027", &rows, &current) {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("unexpected moved-head refusal: {error}").into()),
        Ok(()) => return Err("current identity head reinterpreted old selected output".into()),
    }
    Ok(())
}

fn replace_csv(path: &Path, field: &str, value: &str) -> TestResult {
    let mut reader = csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    let column = header
        .iter()
        .position(|name| name == field)
        .ok_or("CSV column")?;
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    drop(reader);
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record(&header)?;
    for record in records {
        writer.write_record(record.iter().enumerate().map(|(index, text)| {
            if index == column {
                value
            } else {
                text
            }
        }))?;
    }
    writer.flush()?;
    Ok(())
}

fn write_json(path: &Path, value: &serde_json::Value) -> TestResult {
    std::fs::write(path, format!("{}\n", serde_json::to_string(value)?))?;
    Ok(())
}

fn mismatch(result: ReportResult<()>, path: &Path, field: &str) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => {
            check!(detail.starts_with(&format!(
                "{} line 1 does not match the shared best-results reduction:",
                path.display()
            )));
            check!(
                detail.contains(field),
                "readback mismatch should identify {field}: {detail}"
            );
        }
        Err(error) => return Err(format!("unexpected JSONL refusal: {error}").into()),
        Ok(()) => return Err("tampered selected identity passed independent readback".into()),
    }
    Ok(())
}

#[test]
fn independent_selected_bests_readback_rejects_each_current_affiliation_field() -> TestResult {
    let fixture = source_births()?;
    let (dataset, _) = capture(&fixture.store)?;
    let [first, _, _] = &fixture.athletes;
    let rows = [selection(first)];
    let output = fixture.directory.path().join("affiliation-publication");
    for (field, wrong) in [
        ("current_school_id", "foreign-school-id"),
        ("current_school", "Historical School"),
        ("current_state", "AL"),
        ("current_school_as_of", "1900-01-01"),
    ] {
        let (jsonl, csv) = write(
            &output,
            &rows,
            "co2027",
            EventSemantics::ResultGroups21,
            &dataset,
        )?;
        verify(&output, "co2027", &rows, &dataset)?;
        replace_csv(&csv, field, wrong)?;
        check!(verify(&output, "co2027", &rows, &dataset).is_err());
        write(
            &output,
            &rows,
            "co2027",
            EventSemantics::ResultGroups21,
            &dataset,
        )?;
        let mut wire: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&jsonl)?)?;
        let value = wire.get_mut(field).ok_or("current affiliation field")?;
        *value = serde_json::json!(wrong);
        write_json(&jsonl, &wire)?;
        mismatch(verify(&output, "co2027", &rows, &dataset), &jsonl, field)?;
    }
    Ok(())
}

mod canonical_affiliation;
