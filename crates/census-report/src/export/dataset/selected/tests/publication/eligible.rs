use super::{publication, publish, ExportDataset, Fixture, GradYear, Scope, TestResult};
use census_domain::model::{CanonicalAthlete, Gender, SourceIdentity, SourceNamespace};
use census_store::Table;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

mod readback;

#[tokio::test]
async fn missing_window_publication_reconciles_all_artifacts_and_unknown_candidate_audit(
) -> TestResult {
    let fixture = Fixture::composed(true).await?;
    let unknown = append_unknown_candidate(&fixture)?;
    let objects = super::super::temporary()?;
    let publication_root = super::super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    assert_input(&dataset, &fixture)?;
    let candidate = fixture.roster_athlete.to_string();
    let lineage = dataset.lineage.clone();
    let witness = super::super::witness::capture(&dataset)?;
    let root = publication_root.path().join("publication");
    let path = publish(&fixture, &dataset, &root)?;
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    let generation = path.parent().ok_or("generation directory")?;
    let accepted = dataset
        .verified_public_identities()?
        .public_id(fixture.roster_athlete.as_str())
        .ok_or("accepted identity")?
        .as_str()
        .to_owned();
    readback::artifacts(
        generation,
        &path,
        &accepted,
        &candidate,
        unknown.id.as_str(),
    )?;
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; super::super::witness::capture(&reopened.dataset)?, witness);
    let replayed = publication::verify_published(&path)?;
    check!(eq; replayed.generation_digest, verified.generation_digest);
    check!(eq; replayed.workbook.mapped_athletes, 1);
    readback::artifacts(
        generation,
        &path,
        &accepted,
        &candidate,
        unknown.id.as_str(),
    )?;
    Ok(())
}

fn append_unknown_candidate(fixture: &Fixture) -> TestResult<CanonicalAthlete> {
    let unknown = CanonicalAthlete::new(
        &fixture.roster_school,
        "Unaccepted publication audit candidate",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(
            SourceNamespace::Other("unqualified-publication-fixture".into()),
            "unknown-candidate",
        ),
    )?;
    fixture.store.append(Table::Athletes, &unknown)?;
    Ok(unknown)
}

fn assert_input(dataset: &ExportDataset, fixture: &Fixture) -> TestResult {
    check!(eq; dataset.athletes.len(), 2);
    check!(eq; dataset.performances.len(), 2);
    for performance in &dataset.performances {
        let assessment = dataset
            .verified_participation()?
            .assessment(&performance.id)
            .ok_or("participation")?;
        check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Unresolved);
        check!(eq; assessment.reasons().iter().collect::<Vec<_>>(), vec![census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing, census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved]);
        check!(assessment.diagnostics().is_empty());
        check!(eq; performance.athlete, fixture.roster_athlete);
    }
    let outcome = crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: vec![
                census_domain::UsJurisdiction::Alabama,
                census_domain::UsJurisdiction::Wisconsin,
            ],
        },
    )?;
    check!(outcome.selections.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 2);
    check!(eq; crate::bests::mark_text(&fixture.performance.mark), "13-9");
    let second = dataset
        .performances
        .iter()
        .find(|row| row.source_key == "milesplit_result:201782264")
        .ok_or("retained second raw mark")?;
    check!(eq; crate::bests::mark_text(&second.mark), "14-9");
    Ok(())
}

fn csv_rows(path: &Path) -> TestResult<Vec<BTreeMap<String, String>>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect())
        })
        .collect()
}

fn json(path: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
