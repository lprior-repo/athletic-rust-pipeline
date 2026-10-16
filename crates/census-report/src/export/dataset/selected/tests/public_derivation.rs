use super::{Fixture, TestResult};
use crate::report::{self, Derivation, ReportError, Scope};

#[tokio::test]
async fn public_census_derivation_uses_verified_selected_population_and_geography() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    check!(dataset
        .athletes
        .iter()
        .any(|row| row.id != fixture.roster_athlete));
    let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[])?;
    let athlete = derivation.athletes().first().ok_or("selected athlete")?;
    check!(eq; derivation.athletes().len(), 1);
    check!(eq; athlete.id, fixture.roster_athlete);
    check!(eq; athlete.school, fixture.roster_school);
    let census = report::build_census(&derivation, &fixture.store.out_dir());
    check!(eq; census.totals.athletes, 1);
    let (json, csv) = report::write_census(&fixture.store, &census, Scope::AllSources)?;
    let written: serde_json::Value = serde_json::from_slice(&std::fs::read(json)?)?;
    check!(eq; written["totals"]["athletes"], serde_json::json!(1));
    let mut reader = csv::Reader::from_path(csv)?;
    let totals = reader.records().collect::<Result<Vec<_>, _>>()?;
    let total = totals.last().ok_or("census total")?;
    check!(eq; total.get(0), Some("TOTAL"));
    check!(eq; total.get(2), Some("1"));
    Ok(())
}

#[tokio::test]
async fn public_census_refuses_each_missing_selected_view_before_replacing_artifacts() -> TestResult
{
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let mut dataset = fixture.capture(objects.path())?;
    let out = fixture.store.out_dir();
    std::fs::create_dir_all(&out)?;
    let prior = out.join("report.json");
    let prior_csv = out.join("census-by-state.csv");
    std::fs::write(&prior, b"prior census artifact\n")?;
    std::fs::write(&prior_csv, b"prior census CSV\n")?;
    let identity = dataset
        .verified_public_identities
        .take()
        .ok_or("identity view")?;
    refuse_census(
        &dataset,
        &fixture.store,
        "selected publication requires retained public-identity verification",
    )?;
    dataset.verified_public_identities = Some(identity);
    let participation = dataset
        .verified_participation
        .take()
        .ok_or("participation view")?;
    refuse_census(
        &dataset,
        &fixture.store,
        "selected publication requires retained participation verification",
    )?;
    dataset.verified_participation = Some(participation);
    let affiliations = dataset
        .verified_current_affiliations
        .take()
        .ok_or("affiliation view")?;
    refuse_census(
        &dataset,
        &fixture.store,
        "selected publication requires retained current-affiliation verification",
    )?;
    dataset.verified_current_affiliations = Some(affiliations);
    check!(eq; std::fs::read(prior)?.as_slice(), b"prior census artifact\n");
    check!(eq; std::fs::read(prior_csv)?.as_slice(), b"prior census CSV\n");
    check!(eq; Derivation::of(&dataset, Scope::AllSources, None, &[])?.athletes().len(), 1);
    Ok(())
}

fn refuse_census(
    dataset: &crate::export::ExportDataset,
    store: &census_store::Store,
    expected: &str,
) -> TestResult {
    let result = Derivation::of(dataset, Scope::AllSources, None, &[]).and_then(|derivation| {
        let census = report::build_census(&derivation, &store.out_dir());
        report::write_census(store, &census, Scope::AllSources)
    });
    match result {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("unexpected census refusal: {error}").into()),
        Ok(_) => return Err("unverified selected input published a historical census".into()),
    }
    Ok(())
}

#[tokio::test]
async fn bound_empty_outcome_uses_the_shared_selected_serializer() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let output = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[])?;
    let admission = report::EventAdmission::of(&dataset)?;
    let outcome = crate::bests::build_from_derivation(&derivation, &admission, Some(0))?;
    let (jsonl, csv) = crate::bests::write_outcome(
        &output.path().join("bound"),
        &derivation,
        &outcome,
        "selected",
    )?;
    check!(eq; std::fs::read(jsonl)?.len(), 0);
    let mut csv = csv::Reader::from_path(csv)?;
    let headers = csv.headers()?;
    check!(eq; headers.len(), 44);
    check!(eq; headers.iter().skip(39).collect::<Vec<_>>(), vec![
        "canonical_candidate_id", "current_school_id", "current_school",
        "current_state", "current_school_as_of",
    ]);
    check!(headers.iter().any(|name| name == "school_at_performance"));
    check!(headers.iter().any(|name| name == "state_at_performance"));
    check!(!headers
        .iter()
        .any(|name| name == "school" || name == "state"));
    check!(eq; csv.records().count(), 0);
    Ok(())
}
