use super::{publication, ExportDataset, GradYear, Scope, TestResult};
use crate::bests::SelectionOutcome;
use census_domain::model::{
    CanonicalPerformance, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, QueryProfileV1,
};

mod assertions;
mod captures;
mod dependency_loss;
mod fixture;
mod readback;
use fixture::Fixture;

#[tokio::test]
async fn synthetic_michigan_in_season_pr_and_window_fallback_survive_source_deleted_replay(
) -> TestResult {
    let mut fixture = Fixture::synthetic_michigan().await?;
    let objects = super::super::temporary()?;
    let publication_root = super::super::temporary()?;
    let control_objects = super::super::temporary()?;
    let control = fixture.capture(control_objects.path())?;
    assertions::all_retained(&control, &fixture.stronger)?;
    let profile = march_profile()?;
    fixture.select(&profile)?;
    let dataset = fixture.capture(objects.path())?;
    check!(ne; control.selected_profile_id(), dataset.selected_profile_id());
    check!(eq; control.performances, dataset.performances);
    check!(eq; control.meets, dataset.meets);
    let eligible = fixture.eligible.clone();
    let excluded = fixture.stronger.clone();
    let outcome = assertions::bounded(&dataset, &eligible, &excluded, &profile)?;
    for (performance, refs) in [
        (&eligible, &fixture.proofs.eligible),
        (&excluded, &fixture.proofs.excluded),
    ] {
        for captured in [&control, &dataset] {
            check!(eq; captured.verified_participation()?.assessment(&performance.id)
                .ok_or("exact independently expected source proof set")?.evidence_refs(), refs.as_slice());
        }
    }
    let expected = readback::Expected {
        public_id: fixture.public_id.clone(),
        school_id: fixture.school_id.clone(),
        eligible,
        excluded,
        eligible_evidence_refs: fixture.proofs.eligible.clone(),
        evidence_refs: fixture.proofs.excluded.clone(),
        packets: fixture.proofs.packets.clone(),
        input_generation: dataset.lineage.input_generation.clone(),
    };
    let witness = super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let digest = dataset.archive_digest()?;
    let path = fixture.publish(&dataset, &publication_root.path().join("publication"))?;
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
    readback::proofs(&frozen, &expected)?;
    check!(eq; assertions::bounded(&frozen, &expected.eligible, &expected.excluded, &profile)?, outcome);
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; super::super::witness::capture(&reopened.dataset)?, witness);
    readback::proofs(&reopened.dataset, &expected)?;
    check!(eq; assertions::bounded(&reopened.dataset, &expected.eligible, &expected.excluded, &profile)?, outcome);
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    readback::artifacts(generation, &path, &expected)?;
    dependency_loss::rejects(generation, &digest, &expected)?;
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    Ok(())
}

fn march_profile() -> TestResult<QueryProfileV1> {
    let mut input = fixture::profile()?.input().clone();
    input.performance_window = PerformanceWindowV1::Inclusive(ProfileDateRangeV1::new(
        ProfileDate::parse("2026-03-01")?,
        ProfileDate::parse("2026-03-31")?,
    )?);
    Ok(QueryProfileV1::new(input)?)
}
