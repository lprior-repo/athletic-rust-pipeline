use super::*;
use crate::export::public_identity::tests::fixture::{capture, merge, source_births, TestResult};

#[test]
fn unknown_alias_audit_has_no_fabricated_public_key_and_retains_unknown_resolution() -> TestResult {
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [_, _, unknown] = &fixture.athletes;
    let fields = audit(Some(&view), unknown.id.as_str())?.collect::<Vec<_>>();
    check!(eq; fields, vec!["", "unknown", "{\"state\":\"unknown\"}"]);
    check!(eq; audit(None, unknown.id.as_str())?.count(), 0);
    Ok(())
}

#[test]
fn merged_alias_audit_preserves_redirect_revision_and_original_candidate_lookup() -> TestResult {
    let fixture = source_births()?;
    merge(&fixture)?;
    let (view, _) = capture(&fixture.store)?;
    let [_, second, _] = &fixture.athletes;
    let [birth, _] = &fixture.births;
    let fields = audit(Some(&view), second.id.as_str())?.collect::<Vec<_>>();
    check!(eq; fields.first().map(String::as_str), Some(birth.public_id().as_str()));
    check!(eq; fields.get(1).map(String::as_str), Some("redirect"));
    let resolution: serde_json::Value =
        serde_json::from_str(fields.get(2).ok_or("resolution audit")?)?;
    check!(eq; resolution.get("state"), Some(&serde_json::json!("redirect")));
    check!(eq; resolution.get("changed_at"), Some(&serde_json::json!(3)));
    Ok(())
}

mod correction;
use crate::workbook::selected_identity_fixture as fixture;

use crate::report::Scope;
use fixture::selected_dataset;

type AliasRows = BTreeMap<String, BTreeMap<String, String>>;

fn published_aliases(directory: &Path, derivation: &Derivation<'_>) -> TestResult<AliasRows> {
    check!(eq; write(directory, derivation)?, 3);
    let mut reader = csv::Reader::from_path(directory.join("identity_aliases.csv"))?;
    let headers = reader.headers()?.clone();
    check!(!headers.iter().any(|name| name == "resolution_state"));
    check!(headers
        .iter()
        .any(|name| name == "historical_automatic_identity_status"));
    reader
        .records()
        .map(|record| {
            let record = record?;
            let fields: BTreeMap<_, _> = headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect();
            let candidate = fields.get("original_id").ok_or("original alias")?.clone();
            Ok((candidate, fields))
        })
        .collect()
}

#[test]
fn selected_alias_membership_keeps_accepted_births_and_withholds_unknown_candidate() -> TestResult {
    let fixture = source_births()?;
    let dataset = selected_dataset(&fixture.store)?;
    let unchanged = serde_json::to_vec(&dataset)?;
    let derivation = Derivation::for_publication(&dataset, Scope::AllSources, None, &[])?;
    check!(eq; derivation.athletes().len(), 2);
    let rows = published_aliases(fixture.directory.path(), &derivation)?;
    let [first, second, unknown] = &fixture.athletes;
    for (athlete, birth) in [first, second].into_iter().zip(&fixture.births) {
        let row = rows.get(athlete.id.as_str()).ok_or("accepted alias")?;
        check!(eq; row.get("canonical_id").map(String::as_str), Some(athlete.id.as_str()));
        check!(eq; row.get("resolved_id").map(String::as_str), Some(athlete.id.as_str()));
        check!(eq; row.get("public_athlete_id").map(String::as_str), Some(birth.public_id().as_str()));
        check!(eq; row.get("publication_membership").map(String::as_str), Some("canonical_in_publication"));
        check!(eq; row.get("publication_identity_status").map(String::as_str), Some("canonical"));
    }
    let row = rows.get(unknown.id.as_str()).ok_or("unknown alias")?;
    check!(eq; row.get("canonical_id").map(String::as_str), Some("unavailable"));
    check!(eq; row.get("resolved_id").map(String::as_str), Some(unknown.id.as_str()));
    check!(eq; row.get("public_athlete_id").map(String::as_str), Some(""));
    check!(eq; row.get("publication_membership").map(String::as_str), Some("canonical_outside_publication"));
    check!(eq; row.get("publication_identity_status").map(String::as_str), Some("unknown"));
    check!(eq; row.get("identity_resolution").map(String::as_str), Some("{\"state\":\"unknown\"}"));
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    Ok(())
}

#[test]
fn committed_merge_joins_alias_candidate_membership_to_one_retained_public_birth() -> TestResult {
    let fixture = source_births()?;
    merge(&fixture)?;
    let dataset = selected_dataset(&fixture.store)?;
    let unchanged = serde_json::to_vec(&dataset)?;
    let derivation = Derivation::for_publication(&dataset, Scope::AllSources, None, &[])?;
    let [first, second, _] = &fixture.athletes;
    let [birth, _] = &fixture.births;
    let [member] = derivation.athletes() else {
        return Err("committed merge did not publish exactly one candidate group".into());
    };
    check!(eq; member.id, first.id);
    let rows = published_aliases(fixture.directory.path(), &derivation)?;
    for candidate in [first, second] {
        let row = rows.get(candidate.id.as_str()).ok_or("merged alias")?;
        check!(eq; row.get("canonical_id").map(String::as_str), Some(first.id.as_str()));
        check!(eq; row.get("resolved_id").map(String::as_str), Some(first.id.as_str()));
        check!(eq; row.get("public_athlete_id").map(String::as_str), Some(birth.public_id().as_str()));
        check!(eq; row.get("publication_membership").map(String::as_str), Some("canonical_in_publication"));
    }
    let alias = rows.get(second.id.as_str()).ok_or("redirect alias")?;
    check!(eq; alias.get("publication_identity_status").map(String::as_str), Some("redirect"));
    let resolution: serde_json::Value = serde_json::from_str(
        alias
            .get("identity_resolution")
            .ok_or("redirect resolution")?,
    )?;
    check!(eq; resolution.get("changed_at"), Some(&serde_json::json!(3)));
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    Ok(())
}

#[test]
fn historical_alias_schema_keeps_resolution_state_and_has_no_selected_columns() -> TestResult {
    let fixture = source_births()?;
    let dataset = crate::export::ExportDataset::load_public_identity_fixture(&fixture.store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[])?;
    check!(eq; write(fixture.directory.path(), &derivation)?, 3);
    let mut reader = csv::Reader::from_path(fixture.directory.path().join("identity_aliases.csv"))?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), HEADER.to_vec());
    check!(eq; reader.records().collect::<Result<Vec<_>, _>>()?.len(), 3);
    Ok(())
}
