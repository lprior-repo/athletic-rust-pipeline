mod correction;
use super::*;
use crate::report::Scope;
use crate::workbook::{self, publication as published, Censuses, Options};
use census_domain::UsJurisdiction;
use std::path::{Path, PathBuf};

mod artifacts;
mod failures;
mod genuine;
mod historical_build;
mod historical_semantic;
mod omission;
mod retention;
mod roundtrip;
mod setup;
mod transfer;
mod withdrawal;

use setup::{Case, Expected};

fn capture(fixture: &fixture::Fixture, objects: &Path) -> TestResult<ExportDataset> {
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, fixture.profile.id()?);
    let snapshot = fixture.store.snapshot();
    let dataset =
        ExportDataset::from_selected_snapshot(&fixture.store, &snapshot, &selection, objects)?;
    dataset.ensure_current_selected_write()?;
    check!(eq; dataset.lineage.snapshot_sequence, snapshot.sequence());
    check!(eq; dataset.selected_authority()?.qualification_lifecycle(), 7);
    Ok(dataset)
}

fn publish(
    fixture: &fixture::Fixture,
    dataset: &ExportDataset,
    root: &Path,
) -> TestResult<PathBuf> {
    let options = Options {
        grad_year: Some(2027),
        out: Some(root.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: fixture.profile.input().jurisdictions.clone(),
        school_year: fixture.profile.input().contact_school_year,
    };
    let censuses = Censuses::of(dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    Ok(workbook::build_from(
        dataset,
        &fixture.store,
        &options,
        &censuses,
    )?)
}

fn witness(dataset: &ExportDataset, expected: &Expected) -> TestResult<Value> {
    let affiliations = dataset.verified_current_affiliations()?;
    let resolution = affiliations.resolution(&expected.athlete);
    check!(eq; resolution.status(), expected.status());
    check!(eq; resolution.school(), expected.school());
    let identities = dataset.verified_public_identities()?;
    let public = identities
        .public_id(expected.athlete.as_str())
        .ok_or("controlled accepted public identity")?;
    let audit = affiliations
        .audit(&expected.athlete)
        .ok_or("controlled affiliation audit")?;
    check!(!audit.references().is_empty());
    let selected = crate::report::Derivation::selected(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Michigan],
        affiliations,
    )?;
    check!(eq; selected.athletes().len(), expected.selected_count());
    if let Some(school) = expected.school() {
        check!(eq; selected.athletes().first().ok_or("nonempty controlled selected population")?.school, *school);
    } else {
        check!(eq; selected.affiliation_exclusions().get(&expected.athlete), Some(&crate::report::CurrentAffiliationExclusion::Unresolved));
    }
    let outside = crate::report::Derivation::selected(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Wisconsin],
        affiliations,
    )?;
    check!(outside.athletes().is_empty());
    let participation = dataset.verified_participation()?;
    check!(dataset.performances.is_empty());
    Ok(
        json!({"data":dataset,"public":public,"status":resolution.status(),"school":resolution.school(),"references":audit.references(),"gaps":audit.unavailable(),"profile":affiliations.profile_id(),"generation":affiliations.input_generation(),"participation_generation":participation.input_generation(),"identity_head":identities.head()}),
    )
}

fn inspect_wire(path: &Path) -> TestResult<Value> {
    let wire: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    check!(eq; wire.pointer("/selected_proofs/version").and_then(Value::as_u64), Some(3));
    check!(eq; wire.pointer("/selected_proofs/affiliation/version").and_then(Value::as_u64), Some(2));
    check!(eq; wire.pointer("/data/selection/authority/qualification_lifecycle").and_then(Value::as_u64), Some(7));
    check!(wire
        .pointer("/selected_proofs/affiliation/roster_family/digest")
        .and_then(Value::as_str)
        .is_some());
    Ok(wire)
}

fn remove_sources(fixture: fixture::Fixture, objects: tempfile::TempDir) -> TestResult {
    let store_path = fixture.store.root().to_path_buf();
    let objects_path = objects.path().to_path_buf();
    let fixture::Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    directory.close()?;
    objects.close()?;
    check!(!store_path.exists());
    check!(!objects_path.exists());
    Ok(())
}
