use super::*;
use crate::export::public_identity::tests::fixture::{capture, source_births, TestResult};

#[test]
fn selected_output_uses_birth_ids_and_retains_the_candidate_and_resolution() -> TestResult {
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [athlete, _, unknown] = &fixture.athletes;
    let [birth, _] = &fixture.births;
    let identity = Identity::of(Some(&view), athlete.id.as_str())?;
    check!(eq; identity.athlete_id(), birth.public_id().as_str());
    let audit = identity.audit()?.collect::<Vec<_>>();
    check!(eq; audit.first(), Some(&athlete.id.to_string()));
    let resolution: serde_json::Value = serde_json::from_str(audit.get(1).ok_or("resolution")?)?;
    check!(eq; resolution.get("state"), Some(&serde_json::json!("canonical")));
    check!(Identity::of(Some(&view), unknown.id.as_str()).is_err());
    check!(Identity::of(Some(&view), "absent").is_err());
    Ok(())
}

#[test]
fn historical_output_keeps_the_original_key_and_schema() -> TestResult {
    let identity = Identity::of(None, "legacy-candidate")?;
    check!(eq; identity.athlete_id(), "legacy-candidate");
    check!(eq; identity.audit()?.count(), 0);
    check!(eq; headers(&["athlete_id", "name"], None), vec!["athlete_id", "name"]);
    Ok(())
}

#[test]
fn selected_population_refuses_unknown_and_duplicate_public_ids_before_writing() -> TestResult {
    let fixture = source_births()?;
    let mut dataset = ExportDataset::load_public_identity_fixture(&fixture.store)?;
    dataset.bind_selection_for_test(
        census_domain::model::ProfileId::parse(&"a".repeat(64))?,
        census_domain::model::ProfileDate::parse("2026-10-01")?,
    )?;
    dataset.bind_public_identity_fixture_from_store(&fixture.store)?;
    let [first, _, _] = &fixture.athletes;
    preflight(&dataset, std::slice::from_ref(first))?;
    check!(preflight(&dataset, &fixture.athletes).is_err());
    check!(preflight(&dataset, &[first.clone(), first.clone()]).is_err());
    Ok(())
}
