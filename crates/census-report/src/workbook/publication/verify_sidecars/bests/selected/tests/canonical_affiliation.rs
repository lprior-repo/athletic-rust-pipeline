use super::*;
use crate::export::current_school::SelectedSchools;
use crate::export::{
    CurrentAffiliationResolution, VerifiedCurrentAffiliations, VerifiedParticipationView,
};
use census_domain::model::AthleteId;
use std::collections::BTreeMap;

#[test]
fn selected_alias_bests_publish_canonical_affiliation_from_retained_roster_proofs() -> TestResult {
    let fixture = source_births()?;
    merge(&fixture)?;
    let [canonical, alias, _] = &fixture.athletes;
    let rows = [selection(alias)];
    let output = fixture.directory.path().join("canonical-affiliation");
    let (dataset, _proofs) = capture(&fixture.store)?;
    let view = dataset.verified_current_affiliations()?;
    let school_id = view
        .resolution(&canonical.id)
        .school()
        .ok_or("canonical school")?;
    check!(eq; view.resolution(&alias.id), &CurrentAffiliationResolution::Unknown);
    let group = dataset
        .verified_public_identities()?
        .group(alias.id.as_str())
        .ok_or("alias group")?;
    check!(eq; group.canonical_candidate().as_str(), canonical.id.as_str());
    let (jsonl, csv) = write(
        &output,
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &dataset,
    )?;
    let wire: serde_json::Value = serde_json::from_slice(&std::fs::read(&jsonl)?)?;
    let expected = [
        ("canonical_candidate_id", canonical.id.as_str()),
        ("current_school_id", school_id.as_str()),
        ("current_school", "Synthetic Current Wisconsin High"),
        ("current_state", "WI"),
        ("current_school_as_of", "2026-10-01"),
    ];
    for (field, value) in expected {
        check!(eq; wire.get(field).and_then(serde_json::Value::as_str), Some(value));
    }
    check!(eq; wire.get("selection"), Some(&serde_json::to_value(rows.first().ok_or("alias selection")?)?));
    assert_csv(&csv, &expected)?;
    verify(&output, "co2027", &rows, &dataset)?;
    Ok(())
}

#[test]
fn selected_alias_projection_uses_canonical_affiliation_when_alias_is_absent_or_conflicted(
) -> TestResult {
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    crate::export::public_identity::tests::fixture::merge(&fixture)?;
    let [canonical, alias, _] = &fixture.athletes;
    for alias_resolution in [None, Some(CurrentAffiliationResolution::Conflicted)] {
        let (mut dataset, _objects) =
            crate::export::current_school::fixture::capture(&fixture.store)?;
        let current = dataset
            .verified_current_affiliations()?
            .resolution(&canonical.id)
            .clone();
        let school_id = current.school().ok_or("canonical current school")?.clone();
        let mut affiliations = BTreeMap::from([(canonical.id.clone(), current)]);
        if let Some(resolution) = alias_resolution.clone() {
            affiliations.insert(alias.id.clone(), resolution);
        }
        bind_affiliations(&mut dataset, affiliations)?;
        let view = dataset.verified_current_affiliations()?;
        match alias_resolution {
            None => check!(eq; view.resolution(&alias.id), &CurrentAffiliationResolution::Unknown),
            Some(expected) => check!(eq; view.resolution(&alias.id), &expected),
        }
        let selected = SelectedSchools::of(&dataset)?;
        let projected = selected.fields(alias.id.as_str())?;
        check!(eq; projected.current_school_id, school_id.as_str());
        check!(eq; projected.current_school, "Synthetic Current Wisconsin High");
        check!(eq; projected.current_state, "WI");
        check!(eq; projected.current_school_as_of, "2026-10-01");
    }
    Ok(())
}

fn bind_affiliations(
    dataset: &mut ExportDataset,
    rows: BTreeMap<AthleteId, CurrentAffiliationResolution>,
) -> TestResult {
    let profile = dataset.selected_profile_id().ok_or("profile")?.clone();
    let cutoff = dataset.selected_as_of().ok_or("cutoff")?.clone();
    let rule = dataset.selected_participation_rule().ok_or("rule")?.clone();
    let generation = dataset.lineage.input_generation.clone();
    let affiliations =
        VerifiedCurrentAffiliations::fixture(generation.clone(), profile.clone(), cutoff, rows);
    let participation =
        VerifiedParticipationView::fixture(generation, profile, rule, BTreeMap::new());
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(())
}

fn assert_csv(path: &Path, expected: &[(&str, &str)]) -> TestResult {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    let row = records.first().ok_or("alias best CSV row")?;
    for (field, value) in expected {
        let column = headers
            .iter()
            .position(|header| header == *field)
            .ok_or("CSV column")?;
        check!(eq; row.get(column), Some(*value));
    }
    Ok(())
}
