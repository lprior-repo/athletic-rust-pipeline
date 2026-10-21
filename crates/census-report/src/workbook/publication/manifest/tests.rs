use super::{open_publication, verify_published, Artifact, Manifest};
use crate::export::{
    decoded_generations, generation_counter_window, resident_generations, store_lineage,
    ExportDataset,
};
use crate::report::{io_error, ReportResult, Scope};
use crate::workbook::publication::invariant;
use crate::workbook::Options;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod normalized_rows;
mod selected;
mod selection_scope;
mod supporting;

pub(in crate::workbook) fn seed(store: &Store, name: &str) -> TestResult {
    let (mut school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name.to_lowercase(), None);
    let evidence = Evidence::parsed(
        SourceRef::new(
            "milesplit",
            Some("https://www.milesplit.com/athletes/1234".to_string()),
        ),
        "2026-06-01",
    );
    school.evidence.push(evidence.clone());
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1234"),
    )?;
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: evidence.source.clone(),
        });
    athlete.evidence.push(evidence);
    store.append(Table::Athletes, &athlete)?;
    Ok(())
}

pub(in crate::workbook) fn options() -> TestResult<Options> {
    Ok(Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        jurisdictions: Vec::new(),
    })
}

fn repoint_artifact(directory: &Path, name: &str, bytes: &[u8]) -> ReportResult<()> {
    use sha2::Digest;
    let path = directory.join("manifest.json");
    let mut manifest: Manifest = super::super::sidecars::read_json(&path)?;
    let Some(artifact) = manifest.artifacts.get_mut(name) else {
        return Err(invariant(format!("missing generation artifact: {name}")));
    };
    let length = u64::try_from(bytes.len())
        .map_err(|error| invariant(format!("artifact byte count: {error}")))?;
    *artifact = Artifact {
        bytes: length,
        sha256: format!("{:x}", sha2::Sha256::digest(bytes)),
    };
    manifest.generation_digest = manifest.digest()?;
    let encoded = serde_json::to_vec(&manifest)
        .map_err(|error| invariant(format!("encoding generation manifest: {error}")))?;
    std::fs::write(&path, encoded).map_err(|source| io_error(&path, source))?;
    Ok(())
}

#[test]
fn a_stale_store_is_refused_before_the_seal_decodes_the_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed(&store, "Publishing High School")?;
    let published = crate::workbook::build(&store, &options()?)?;
    let (later, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Later School",
        "later school",
        None,
    );
    store.append(Table::Schools, &later)?;
    let stale = store_lineage(&store)?;
    let _counter_window = generation_counter_window()?;
    let decoded_baseline = decoded_generations();
    let error = match open_publication(&published, &stale, Scope::AllSources, GradYear::CO2027) {
        Err(error) => error,
        Ok(_) => return Err("a stale store generation opened the publication".into()),
    };
    check!(
        error.to_string().contains("publication is stale"),
        "the refusal names the stale store generation: {error}"
    );
    check!(
        eq;
        decoded_generations(),
        decoded_baseline,
        "the stale store generation was refused before any table decode"
    );
    Ok(())
}

#[test]
fn a_seal_opens_one_generation_that_carries_the_published_frozen_input() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed(&store, "Publishing High School")?;
    let published = crate::workbook::build(&store, &options()?)?;
    let generation = published.parent().ok_or("missing generation directory")?;
    let lineage = store_lineage(&store)?;
    let _counter_window = generation_counter_window()?;
    let baseline = resident_generations();
    let decoded_baseline = decoded_generations();
    let opened = open_publication(&published, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; opened.workbook.mapped_athletes, 1);
    check!(
        eq;
        decoded_generations(),
        decoded_baseline.saturating_add(1),
        "the seal opened exactly one complete generation"
    );
    check!(
        eq;
        resident_generations(),
        baseline.saturating_add(1),
        "one complete generation is resident for verification and measurement"
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let captured = manifest
        .get("lineage")
        .and_then(|lineage| lineage.get("input_digest"))
        .and_then(serde_json::Value::as_str)
        .ok_or("missing manifest input digest")?;
    check!(
        eq;
        opened.dataset.lineage.input_digest,
        captured,
        "the opened generation is the one the manifest captured"
    );
    let digest = manifest
        .get("generation_digest")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing generation digest")?;
    check!(
        eq;
        opened.generation_digest,
        digest,
        "the seal names the generation it verified"
    );
    drop(opened);
    check!(
        eq;
        resident_generations(),
        baseline,
        "the generation is released when the publication is dropped"
    );
    Ok(())
}

#[test]
fn a_seal_refuses_a_generation_whose_frozen_input_names_another_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed(&store, "Publishing High School")?;
    let published = crate::workbook::build(&store, &options()?)?;
    let generation = published.parent().ok_or("missing generation directory")?;

    let other = Store::open(directory.path().join("other-store"))?;
    seed(&other, "Foreign High School")?;
    let foreign_dataset = ExportDataset::load(&other)?;
    let foreign_lineage = foreign_dataset.lineage.clone();
    let foreign_path = directory.path().join("foreign-frozen.json");
    foreign_dataset.save_frozen(&foreign_path)?;
    drop(foreign_dataset);
    let foreign_bytes = std::fs::read(&foreign_path)?;
    std::fs::write(generation.join("frozen-input.json"), &foreign_bytes)?;
    repoint_artifact(generation, "frozen-input.json", &foreign_bytes)?;

    let lineage = store_lineage(&store)?;
    check!(
        ne;
        foreign_lineage,
        lineage,
        "the substituted frozen input names another generation"
    );
    let error = match open_publication(&published, &lineage, Scope::AllSources, GradYear::CO2027) {
        Err(error) => error,
        Ok(_) => {
            return Err(
                "the seal accepted a generation whose frozen input names another lineage".into(),
            )
        }
    };
    check!(
        error
            .to_string()
            .contains("manifest and frozen input lineage disagree"),
        "the seal decodes the frozen input and refuses the lineage it names: {error}"
    );
    let error = match verify_published(&published) {
        Err(error) => error,
        Ok(_) => return Err("the offline route accepted a foreign frozen input".into()),
    };
    check!(
        error
            .to_string()
            .contains("manifest and frozen input lineage disagree"),
        "the offline route refuses the same generation: {error}"
    );
    Ok(())
}

#[test]
fn seal_verification_shares_one_generation_across_exact_readback() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Publishing High School")?;
    let published = crate::workbook::build(&store, &options()?)?;
    let lineage = store_lineage(&store)?;
    let _counter_window = generation_counter_window()?;
    let decoded = decoded_generations();
    let resident = resident_generations();
    let opened = open_publication(&published, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; opened.workbook.mapped_athletes, 1);
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("decode counter overflow")?);
    check!(eq; resident_generations(), resident.checked_add(1).ok_or("resident counter overflow")?);
    let readback =
        crate::workbook::verify::verify_frozen(&published, &opened.dataset, &options()?)?;
    check!(eq; readback.mapped_athletes, opened.workbook.mapped_athletes);
    check!(eq; readback.rows, opened.workbook.rows);
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("decode counter overflow")?);
    let bytes = std::fs::read(&published)?;
    let digest = opened.generation_digest;
    drop(opened.dataset);
    let independent = verify_published(&published)?;
    check!(eq; independent.generation_digest, digest);
    check!(eq; independent.workbook.mapped_athletes, readback.mapped_athletes);
    check!(eq; std::fs::read(&published)?, bytes);
    Ok(())
}

#[test]
fn selection_canonicalizes_jurisdiction_order_and_duplicates() -> TestResult {
    let mut first = options()?;
    first.jurisdictions = vec![UsJurisdiction::Wisconsin, UsJurisdiction::Ohio];
    let mut second = options()?;
    second.jurisdictions = vec![
        UsJurisdiction::Ohio,
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Ohio,
    ];
    check!(eq; super::Selection::of(&first), super::Selection::of(&second));
    Ok(())
}
