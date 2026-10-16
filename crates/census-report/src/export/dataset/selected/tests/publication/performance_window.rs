use super::{publication, publish, ExportDataset, Fixture, GradYear, Scope, TestResult};
use crate::bests::SelectionOutcome;
use census_domain::model::{
    CanonicalPerformance, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, ProfileSelectionV1,
    QueryProfileV1,
};

mod assertions;
mod captures;
mod readback;

#[tokio::test]
async fn configured_query_window_retains_missing_status_window_and_exact_outside_window_exclusions(
) -> TestResult {
    let mut fixture = Fixture::composed(true).await?;
    let objects = super::super::temporary()?;
    let publication_root = super::super::temporary()?;
    let control_objects = super::super::temporary()?;
    let control = fixture.capture(control_objects.path())?;
    assertions::all_retained(&control)?;
    let profile = march_profile()?;
    fixture.store.register_profile(&profile)?;
    fixture.selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
    let dataset = fixture.capture(objects.path())?;
    check!(ne; control.selected_profile_id(), dataset.selected_profile_id());
    check!(eq; control.performances, dataset.performances);
    check!(eq; control.meets, dataset.meets);
    let unresolved = fixture.performance.clone();
    let excluded = dataset
        .performances
        .iter()
        .find(|row| row.source_key == "milesplit_result:201782264")
        .ok_or("stronger retained high-school result")?
        .clone();
    let outcome = assertions::bounded(&dataset, &unresolved, &excluded, &profile)?;
    let assessment = dataset
        .verified_participation()?
        .assessment(&excluded.id)
        .ok_or("window exclusion assessment")?;
    let expected = readback::Expected {
        public_id: dataset
            .verified_public_identities()?
            .public_id(unresolved.athlete.as_str())
            .ok_or("accepted public identity")?
            .as_str()
            .to_owned(),
        unresolved,
        excluded,
        unresolved_evidence_refs: dataset
            .verified_participation()?
            .assessment(&fixture.performance.id)
            .ok_or("unresolved assessment")?
            .evidence_refs()
            .to_vec(),
        evidence_refs: assessment.evidence_refs().to_vec(),
        input_generation: dataset.lineage.input_generation.clone(),
    };
    let witness = super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let digest = dataset.archive_digest()?;
    let path = publish(
        &fixture,
        &dataset,
        &publication_root.path().join("publication"),
    )?;
    let generation = path.parent().ok_or("publication generation")?;
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    readback::artifacts(generation, &path, &expected)?;
    let store_path = fixture.store.root().to_path_buf();
    let cache_path = fixture.store.http_cache_dir();
    let staging_path = objects.path().to_path_buf();
    fixture.remove_sources()?;
    objects.close()?;
    drop(control);
    control_objects.close()?;
    drop(dataset);
    check!(!store_path.exists());
    check!(!cache_path.exists());
    check!(!staging_path.exists());
    let frozen =
        ExportDataset::reopen_frozen_with_digest(&generation.join("frozen-input.json"), &digest)?;
    check!(eq; super::super::witness::capture(&frozen)?, witness);
    check!(eq; assertions::bounded(&frozen, &expected.unresolved, &expected.excluded, &profile)?, outcome);
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; super::super::witness::capture(&reopened.dataset)?, witness);
    check!(eq; assertions::bounded(&reopened.dataset, &expected.unresolved, &expected.excluded, &profile)?, outcome);
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    readback::artifacts(generation, &path, &expected)?;
    Ok(())
}

fn march_profile() -> TestResult<QueryProfileV1> {
    let mut input = super::super::fixture::profile()?.input().clone();
    input.performance_window = PerformanceWindowV1::Inclusive(ProfileDateRangeV1::new(
        ProfileDate::parse("2026-03-01")?,
        ProfileDate::parse("2026-03-31")?,
    )?);
    Ok(QueryProfileV1::new(input)?)
}
