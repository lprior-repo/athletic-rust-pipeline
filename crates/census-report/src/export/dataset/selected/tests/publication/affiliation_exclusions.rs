use super::{
    publication, workbook, ExportDataset, GradYear, Options, SchoolYear, Scope, TestResult,
    UsJurisdiction,
};
use crate::export::public_identity::tests::fixture::Fixture;
use crate::export::selected_test_fixture::consumer::{self, RosterCase};
use crate::export::CurrentAffiliationResolution;
use census_domain::model::SourceNamespace;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

mod readback;
mod rows;

struct Expected {
    first: String,
    second: String,
    unknown: String,
    first_public: String,
    second_public: String,
    roster: BTreeSet<String>,
    schools: BTreeMap<String, String>,
    references: Vec<String>,
    profile: String,
    generation: String,
    retained: BTreeMap<&'static str, Value>,
}

#[test]
fn conflicting_retained_rosters_publish_exclusions_and_replay_without_sources() -> TestResult {
    let fixture = consumer::source_births_with(RosterCase::Conflicted)?;
    let (dataset, objects) = consumer::capture(&fixture.store)?;
    let expected = capture(&fixture, &dataset)?;
    let original = serde_json::to_value(&dataset)?;
    let witness = super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let publication_root = super::super::temporary()?;
    let options = Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: Some(publication_root.path().join("publication")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: SchoolYear::new(2026).ok_or("publication year")?,
    };
    let censuses =
        workbook::Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let path = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 0);
    check!(eq; serde_json::to_value(&dataset)?, original);
    let generation = path.parent().ok_or("published generation")?;
    readback::artifacts(generation, &path, &expected)?;
    let source_root = fixture.store.root().to_path_buf();
    let source_cache = fixture.store.http_cache_dir();
    let source_objects = objects.path().to_path_buf();
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    directory.close()?;
    objects.close()?;
    drop(dataset);
    check!(!source_root.exists());
    check!(!source_cache.exists());
    check!(!source_objects.exists());
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; super::super::witness::capture(&reopened.dataset)?, witness);
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    readback::artifacts(generation, &path, &expected)
}

fn capture(fixture: &Fixture, dataset: &ExportDataset) -> TestResult<Expected> {
    check!(eq; dataset.athletes.len(), 5);
    check!(dataset.performances.is_empty());
    let [first, second, unknown] = &fixture.athletes;
    let [first_birth, second_birth] = &fixture.births;
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_ids().count(), 2);
    check!(eq; identities.public_id(first.id.as_str()), Some(first_birth.public_id()));
    check!(eq; identities.public_id(second.id.as_str()), Some(second_birth.public_id()));
    check!(identities.public_id(unknown.id.as_str()).is_none());
    let roster: BTreeSet<_> = dataset
        .athletes
        .iter()
        .filter(|athlete| {
            !fixture
                .athletes
                .iter()
                .any(|original| original.id == athlete.id)
        })
        .map(|athlete| athlete.id.to_string())
        .collect();
    check!(eq; roster.len(), 2);
    let mut members = roster.clone();
    check!(members.insert(first.id.to_string()));
    let group = identities
        .group(first.id.as_str())
        .ok_or("first public group")?;
    check!(eq; group.canonical_candidate().as_str(), first.id.as_str());
    check!(eq; group.members().map(|(id, _)| id.to_string()).collect::<BTreeSet<_>>(), members);
    for candidate in &roster {
        check!(eq; identities.public_id(candidate), Some(first_birth.public_id()));
    }
    let references = audit(dataset, fixture, &roster)?;
    let affiliations = dataset.verified_current_affiliations()?;
    let retained = BTreeMap::from([
        ("athletes", serde_json::to_value(&dataset.athletes)?),
        ("schools", serde_json::to_value(&dataset.schools)?),
        ("performances", serde_json::to_value(&dataset.performances)?),
    ]);
    Ok(Expected {
        first: first.id.to_string(),
        second: second.id.to_string(),
        unknown: unknown.id.to_string(),
        first_public: first_birth.public_id().as_str().to_owned(),
        second_public: second_birth.public_id().as_str().to_owned(),
        roster,
        schools: schools(dataset)?,
        references,
        profile: affiliations.profile_id().as_str().to_owned(),
        generation: dataset.lineage.input_generation.clone(),
        retained,
    })
}

fn audit(
    dataset: &ExportDataset,
    fixture: &Fixture,
    roster: &BTreeSet<String>,
) -> TestResult<Vec<String>> {
    let [first, second, unknown] = &fixture.athletes;
    let affiliations = dataset.verified_current_affiliations()?;
    check!(eq; affiliations.as_of().as_str(), "2026-10-01");
    check!(eq; affiliations.resolution(&first.id), &CurrentAffiliationResolution::Conflicted);
    let conflict = affiliations.audit(&first.id).ok_or("conflict audit")?;
    check!(eq; conflict.references().len(), 4);
    check!(eq; conflict.unavailable(), &["no_retained_row".to_owned()]);
    for (athlete, reason) in [
        (second, "no_retained_row"),
        (unknown, "identity_unaccepted"),
    ] {
        check!(eq; affiliations.resolution(&athlete.id), &CurrentAffiliationResolution::Unknown);
        let audit = affiliations.audit(&athlete.id).ok_or("unknown audit")?;
        check!(audit.references().is_empty());
        check!(eq; audit.unavailable(), &[reason.to_owned()]);
    }
    for athlete in dataset
        .athletes
        .iter()
        .filter(|athlete| roster.contains(athlete.id.as_str()))
    {
        check!(eq; affiliations.resolution(&athlete.id), &CurrentAffiliationResolution::Unknown);
        check!(affiliations.audit(&athlete.id).is_none());
        let evidence = athlete.evidence.first().ok_or("retained roster evidence")?;
        let note: Value = serde_json::from_str(evidence.note.as_deref().ok_or("roster note")?)?;
        let key = note
            .get("source_row_key")
            .and_then(Value::as_str)
            .ok_or("retained row key")?;
        check!(conflict
            .references()
            .iter()
            .any(|reference| reference == key));
        let digest = note
            .get("capture_sha256")
            .and_then(Value::as_str)
            .ok_or("capture digest")?;
        let prefix = format!("archive/captures/{digest}/");
        check!(eq; conflict.references().iter().filter(|reference| reference.starts_with(&prefix)).count(), 1);
        check!(eq; note.get("fetched_at").and_then(Value::as_str), Some("2026-10-01T12:00:00Z"));
        check!(eq; note.get("provider_athlete_id").and_then(Value::as_str), Some("9401"));
    }
    Ok(conflict.references().to_vec())
}

fn schools(dataset: &ExportDataset) -> TestResult<BTreeMap<String, String>> {
    check!(eq; dataset.schools.len(), 2);
    let mut schools = BTreeMap::new();
    let mut owners = BTreeSet::new();
    for school in dataset.schools.values() {
        check!(eq; school.state, Some(UsJurisdiction::Wisconsin));
        check!(!school.co_op);
        check!(eq; school.source_identities.len(), 1);
        let owner = school.source_identities.first().ok_or("school owner")?;
        check!(eq; owner.namespace, SourceNamespace::MilesplitSchool);
        check!(owners.insert(owner.id.as_str()));
        check!(schools
            .insert(school.id.to_string(), school.name.clone())
            .is_none());
    }
    check!(eq; owners, BTreeSet::from(["52649", "52650"]));
    check!(eq; schools.values().map(String::as_str).collect::<BTreeSet<_>>(), BTreeSet::from([
        "Synthetic Current Wisconsin High", "Synthetic Conflicting Wisconsin High",
    ]));
    Ok(schools)
}
