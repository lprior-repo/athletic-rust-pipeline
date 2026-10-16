use super::TestResult;
use crate::workbook::publication::verify_sidecars::input::Inputs;
use crate::workbook::publication::{normalized, selected::fixture::Fixture};

#[test]
fn selected_normalized_public_ids_roundtrip_and_reject_candidate_fallback_or_resolution_changes(
) -> TestResult {
    let fixture = Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let outcome = inputs.outcome()?;
    let directory = tempfile::tempdir()?;
    let candidate = fixture
        .dataset
        .athletes
        .first()
        .ok_or("candidate")?
        .id
        .as_str();
    let public_id = fixture
        .dataset
        .verified_public_identities()?
        .public_id(candidate)
        .ok_or("public identity")?;
    for (column, value) in [
        ("athlete_id", candidate),
        ("canonical_candidate_id", "foreign-candidate"),
        ("identity_resolution", "{\"state\":\"unknown\"}"),
    ] {
        normalized::write_outcome(
            directory.path(),
            &inputs.derivation,
            &fixture.options,
            &outcome,
        )?;
        super::super::verify_outcome(directory.path(), &inputs, 7, &outcome)?;
        let path = directory.path().join("athletes.csv");
        let mut reader = csv::Reader::from_path(&path)?;
        let headers = reader.headers()?.clone();
        let mut records = reader.records().collect::<Result<Vec<_>, _>>()?;
        let id = headers
            .iter()
            .position(|name| name == "athlete_id")
            .ok_or("public ID column")?;
        check!(eq; records.first().and_then(|row| row.get(id)), Some(public_id.as_str()));
        let index = headers
            .iter()
            .position(|name| name == column)
            .ok_or("audit column")?;
        let first = records.first_mut().ok_or("athlete row")?;
        let fields: Vec<_> = first
            .iter()
            .enumerate()
            .map(|(position, field)| {
                if position == index {
                    value.to_owned()
                } else {
                    field.to_owned()
                }
            })
            .collect();
        *first = csv::StringRecord::from(fields);
        drop(reader);
        let mut writer = csv::Writer::from_path(&path)?;
        writer.write_record(&headers)?;
        records
            .iter()
            .try_for_each(|row| writer.write_record(row))?;
        writer.flush()?;
        check!(super::super::verify_outcome(directory.path(), &inputs, 7, &outcome).is_err());
    }
    Ok(())
}

#[test]
fn selected_alias_audit_retains_candidate_keys_separately_from_public_identity() -> TestResult {
    let fixture = Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let outcome = inputs.outcome()?;
    let directory = tempfile::tempdir()?;
    normalized::write_outcome(
        directory.path(),
        &inputs.derivation,
        &fixture.options,
        &outcome,
    )?;
    let mut reader = csv::Reader::from_path(directory.path().join("identity_aliases.csv"))?;
    let header = reader.headers()?.clone();
    let record = reader.records().next().ok_or("alias row")??;
    let field = |name| -> TestResult<&str> {
        record
            .get(
                header
                    .iter()
                    .position(|column| column == name)
                    .ok_or("alias column")?,
            )
            .ok_or_else(|| "alias cell".into())
    };
    let candidate = fixture
        .dataset
        .athletes
        .first()
        .ok_or("candidate")?
        .id
        .as_str();
    check!(eq; field("original_id")?, candidate);
    check!(eq; field("resolved_id")?, candidate);
    check!(ne; field("public_athlete_id")?, candidate);
    check!(eq; field("publication_identity_status")?, "canonical");
    let resolution: serde_json::Value = serde_json::from_str(field("identity_resolution")?)?;
    check!(eq; resolution.get("state"), Some(&serde_json::json!("canonical")));
    Ok(())
}

#[test]
fn selected_populated_best_marks_rejects_candidate_fallback_and_changed_identity_audit(
) -> TestResult {
    let fixture = crate::workbook::public_identity::fixture::Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let outcome = inputs.outcome()?;
    check!(eq; outcome.selections.len(), 1);
    let candidate = fixture
        .dataset
        .athletes
        .first()
        .ok_or("candidate")?
        .id
        .as_str();
    let public_id = fixture
        .dataset
        .verified_public_identities()?
        .public_id(candidate)
        .ok_or("public identity")?;
    let directory = tempfile::tempdir()?;
    for (column, replacement) in [
        ("athlete_id", candidate),
        ("canonical_candidate_id", "foreign-candidate"),
        ("identity_resolution", "{\"state\":\"unknown\"}"),
    ] {
        normalized::write_outcome(
            directory.path(),
            &inputs.derivation,
            &fixture.options,
            &outcome,
        )?;
        super::super::verify_outcome(directory.path(), &inputs, 7, &outcome)?;
        let path = directory.path().join("best_marks.csv");
        let mut reader = csv::Reader::from_path(&path)?;
        let headers = reader.headers()?.clone();
        let record = reader.records().next().ok_or("best mark")??;
        check!(eq; record.get(0), Some(public_id.as_str()));
        let index = headers
            .iter()
            .position(|name| name == column)
            .ok_or("identity column")?;
        let fields: Vec<_> = record
            .iter()
            .enumerate()
            .map(|(position, field)| {
                if position == index {
                    replacement.to_owned()
                } else {
                    field.to_owned()
                }
            })
            .collect();
        drop(reader);
        let mut writer = csv::Writer::from_path(&path)?;
        writer.write_record(&headers)?;
        writer.write_record(&fields)?;
        writer.flush()?;
        check!(super::super::verify_outcome(directory.path(), &inputs, 7, &outcome).is_err());
    }
    Ok(())
}
