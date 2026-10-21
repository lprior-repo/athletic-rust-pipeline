use super::{witness, ExportDataset, Fixture, TestResult};
use crate::report::Scope;
use crate::workbook::{self, publication, Censuses, Options};
use census_domain::model::{GradYear, SchoolYear};
use census_domain::UsJurisdiction;
use std::path::{Path, PathBuf};

#[tokio::test]
async fn genuine_selected_workbook_publishes_and_verifies_without_original_sources() -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let publication_root = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let expected = witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let root = publication_root.path().join("publication");
    let path = publish(&fixture, &dataset, &root)?;
    let verified = publication::verify_published(&path)?;
    check!(verified.workbook.mapped_athletes > 0);
    check!(eq; publication::current_workbook(&root)?, path);
    check!(matches!(
        publication::recover_publication(&fixture.store, &root)?,
        publication::RecoveryOutcome::HistoricalSuccess { .. }
    ));
    let generation = path.parent().ok_or("published generation")?;
    check!(generation.join("current_affiliations.csv").is_file());
    check!(generation.join("participation_exclusions.csv").is_file());
    check!(generation.join("frozen-input.json.proofs").is_dir());
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; reopened.generation_digest, verified.generation_digest);
    check!(eq; witness::capture(&reopened.dataset)?, expected);
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    Ok(())
}

#[tokio::test]
async fn published_selected_proof_loss_and_tampering_fail_verification_and_recovery() -> TestResult
{
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let publication_root = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let root = publication_root.path().join("publication");
    let path = publish(&fixture, &dataset, &root)?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let generation = path.parent().ok_or("published generation")?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let workbook = std::fs::read(&path)?;
    let proofs = generation.join("frozen-input.json.proofs");
    let object = std::fs::read_dir(&proofs)?
        .next()
        .ok_or("published proof")??
        .path();
    let original = std::fs::read(&object)?;
    std::fs::remove_file(&object)?;
    check!(publication::verify_published(&path).is_err());
    check!(publication::current_workbook(&root).is_err());
    check!(publication::recover_publication(&fixture.store, &root).is_err());
    check!(!object.exists());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; std::fs::read(generation.join("manifest.json"))?, manifest);
    check!(eq; std::fs::read(&path)?, workbook);
    std::fs::write(&object, &original)?;
    publication::verify_published(&path)?;
    let mut tampered = original.clone();
    let first = tampered.first_mut().ok_or("nonempty proof")?;
    *first ^= 1;
    std::fs::write(&object, &tampered)?;
    check!(publication::verify_published(&path).is_err());
    check!(publication::current_workbook(&root).is_err());
    check!(publication::recover_publication(&fixture.store, &root).is_err());
    check!(eq; std::fs::read(&object)?, tampered);
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; std::fs::read(generation.join("manifest.json"))?, manifest);
    check!(eq; std::fs::read(&path)?, workbook);
    Ok(())
}

fn publish(fixture: &Fixture, dataset: &ExportDataset, root: &Path) -> TestResult<PathBuf> {
    let options = Options {
        grad_year: Some(2027),
        out: Some(root.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: SchoolYear::new(2026).ok_or("publication year")?,
    };
    let censuses = Censuses::of(dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    Ok(workbook::build_from(
        dataset,
        &fixture.store,
        &options,
        &censuses,
    )?)
}

pub(crate) mod controlled_three_marks;
mod eligible;

mod mixed;

mod performance_window;

mod transfer;

mod crash;

mod selected_scope;

mod identity_cutover;

mod transfer_geography;

mod meet_geography;

mod affiliation_exclusions;

mod documentary;
