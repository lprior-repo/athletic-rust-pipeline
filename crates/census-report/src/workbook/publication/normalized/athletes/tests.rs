use super::*;
use crate::export::{CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations};
use crate::report::Scope;
use census_domain::model::{
    CanonicalSchool, Evidence, GradYear, ProfileDate, ProfileId, SourceIdentity, SourceNamespace,
    SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<(
    tempfile::TempDir,
    ExportDataset,
    census_domain::model::SchoolId,
)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let current = populate(&store)?;
    Ok((directory, ExportDataset::load(&store)?, current))
}

fn selected_fixture() -> TestResult<(
    tempfile::TempDir,
    Store,
    ExportDataset,
    census_domain::model::SchoolId,
)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let current = populate(&store)?;
    let athletes = store.snapshot().athletes()?;
    let athlete = athletes.first().ok_or("synthetic transfer athlete")?;
    writer.establish_source_bound(
        &store,
        "synthetic-transfer-birth",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let dataset = ExportDataset::load_public_identity_fixture(&store)?;
    Ok((directory, store, dataset, current))
}

fn populate(store: &Store) -> TestResult<census_domain::model::SchoolId> {
    let (old_school, old) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Old High", "old high", None);
    let (new_school, current) = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        "Current High",
        "current high",
        None,
    );
    store.append(Table::Schools, &old_school)?;
    store.append(Table::Schools, &new_school)?;
    let mut athlete = CanonicalAthlete::new(
        &old,
        "Transferred athlete",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9701"),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic-transfer-fixture",
            Some("https://synthetic-selected.test/athletes/9701".into()),
        ),
        "2026-10-14",
    ));
    store.append(Table::Athletes, &athlete)?;
    Ok(current)
}

fn emitted(
    directory: &Path,
    derivation: &Derivation<'_>,
) -> TestResult<std::collections::BTreeMap<String, String>> {
    check!(eq; super::write(directory, derivation, &[])?, 1);
    let mut csv = csv::Reader::from_path(directory.join("athletes.csv"))?;
    let header = csv.headers()?.clone();
    let mut records = csv.records();
    let record = records.next().ok_or("athlete row")??;
    check!(records.next().transpose()?.is_none());
    Ok(header
        .iter()
        .zip(record.iter())
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect())
}

#[test]
fn selected_athlete_csv_retains_bound_current_school_status_and_evidence() -> TestResult {
    let (directory, store, mut dataset, current) = selected_fixture()?;
    let profile = ProfileId::parse(&"a".repeat(64))?;
    let cutoff = ProfileDate::parse("2026-10-01")?;
    dataset.bind_selection_for_test(profile.clone(), cutoff.clone())?;
    dataset.bind_public_identity_fixture_from_store(&store)?;
    let athlete = dataset.athletes.first().ok_or("athlete")?;
    let public_id = dataset
        .verified_public_identities()?
        .public_id(athlete.id.as_str())
        .ok_or("retained public identity")?;
    let refs = vec![
        "retained-affiliation-one".to_owned(),
        "retained-affiliation-two".to_owned(),
    ];
    let view = VerifiedCurrentAffiliations::fixture(
        dataset.lineage.input_generation.clone(),
        profile,
        cutoff,
        [(
            athlete.id.clone(),
            CurrentAffiliationResolution::Resolved {
                school: current.clone(),
                evidence_refs: refs.clone(),
            },
        )]
        .into_iter()
        .collect(),
    );
    let unchanged = serde_json::to_vec(&dataset)?;
    let selected = Derivation::selected(
        &dataset,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Michigan],
        &view,
    )?;
    let row = emitted(directory.path(), &selected)?;
    check!(eq; row.get("athlete_id").map(String::as_str), Some(public_id.as_str()));
    check!(eq; row.get("identity_status").map(String::as_str), Some("accepted_public_identity"));
    check!(eq; row.get("canonical_candidate_id").map(String::as_str), Some(athlete.id.as_str()));
    check!(ne; public_id.as_str(), athlete.id.as_str());
    check!(eq; row.get("current_school_id"), Some(&current.to_string()));
    check!(eq; row.get("current_school_status").map(String::as_str), Some("resolved_at_selected_cutoff"));
    check!(eq; row.get("affiliation_evidence_refs"), Some(&serde_json::to_string(&refs)?));
    check!(eq; row.get("affiliation_evidence_status").map(String::as_str), Some("retained_selected_affiliation_evidence"));
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    Ok(())
}

#[test]
fn historical_athlete_csv_keeps_exact_affiliation_literals_for_retained_policies() -> TestResult {
    let (directory, mut dataset, _) = fixture()?;
    for policy in 18..=22 {
        dataset.bind_historical_policy_for_test(policy)?;
        let historical = Derivation::of(
            &dataset,
            Scope::AllSources,
            None,
            &[UsJurisdiction::Wisconsin],
        )?;
        let row = emitted(directory.path(), &historical)?;
        let athlete = historical.athletes().first().ok_or("historical athlete")?;
        let status = dataset.identities().status(athlete.id.as_str())?;
        check!(eq; row.get("identity_status").map(String::as_str), Some(status.as_str()));
        check!(!row.contains_key("canonical_candidate_id"));
        check!(eq; row.get("current_school_status").map(String::as_str), Some("canonical_school_currentness_unavailable"));
        check!(eq; row.get("affiliation_evidence_refs").map(String::as_str), Some("unavailable"));
        check!(eq; row.get("affiliation_evidence_status").map(String::as_str), Some("claim_level_affiliation_links_not_retained"));
    }
    Ok(())
}
