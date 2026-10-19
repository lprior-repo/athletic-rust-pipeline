use super::super::super::TestResult;
use census_domain::model::CanonicalPerformance;
use census_domain::public_identity_history::PublicAthleteId;
use census_report::export::ExportDataset;
use std::collections::BTreeMap;
use std::path::Path;

mod fixture;
mod identity_history;
mod replay;
use fixture::Fixture;
use replay::RetainedGeneration;

fn build_prior(
    fixture: &Fixture,
    options: &crate::workbook::Options,
) -> TestResult<RetainedGeneration> {
    let request = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let path = super::super::super::build(
        &fixture.store,
        options,
        "report-source-service-before-split",
        &request,
    )?
    .path;
    super::super::super::check_selected_output(
        Path::new(&path),
        &fixture.public_id,
        fixture.eligible.athlete.as_str(),
        &fixture.school_id,
        "14-9",
    )?;
    RetainedGeneration::capture(path)
}

fn split_successors(
    fixture: &Fixture,
    old_public: &str,
    case_id: &str,
    canonical: &PublicAthleteId,
) -> TestResult<BTreeMap<String, String>> {
    let successors = identity_history::split(fixture, case_id, canonical)?;
    let successors: BTreeMap<String, String> = successors
        .into_iter()
        .map(|(candidate, public)| (candidate, public.as_str().to_owned()))
        .collect();
    check!(eq; successors.len(), 2);
    check!(eq; successors.values().collect::<std::collections::BTreeSet<_>>().len(), 2);
    check!(successors.values().all(|public| public != old_public));
    let old_public_id = PublicAthleteId::parse(old_public)?;
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("post-split accepted identity history")?;
    let resolution = accepted.resolve_public(&old_public_id)?;
    let census_domain::public_identity_history::PublicResolution::Ambiguous {
        successors: rows,
        unresolved_bindings: false,
        ..
    } = resolution
    else {
        return Err("post-split historical ID was not ambiguous".into());
    };
    let historical_successors: std::collections::BTreeSet<_> = rows
        .into_iter()
        .map(|row| row.public_id.as_str().to_owned())
        .collect();
    check!(eq;
        historical_successors,
        successors.values().cloned().collect::<std::collections::BTreeSet<_>>()
    );
    Ok(successors)
}

fn assert_aliases(generation: &Path, successors: &BTreeMap<String, String>) -> TestResult {
    let mut aliases = csv::Reader::from_path(generation.join("identity_aliases.csv"))?;
    let headers = aliases.headers()?.clone();
    let candidate_column = headers
        .iter()
        .position(|value| value == "original_id")
        .ok_or("identity alias candidate column")?;
    let public_column = headers
        .iter()
        .position(|value| value == "public_athlete_id")
        .ok_or("identity alias public ID column")?;
    let alias_rows = aliases.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; alias_rows.len(), successors.len());
    for (candidate, public) in successors {
        check!(
            alias_rows.iter().any(|row| {
                row.get(candidate_column) == Some(candidate.as_str())
                    && row.get(public_column) == Some(public.as_str())
            }),
            "post-split service alias missing: {candidate} -> {public}"
        );
    }
    Ok(())
}

fn assert_retry(
    store: &census_store::Store,
    options: &crate::workbook::Options,
    selection: &census_domain::model::ProfileSelectionV1,
    current: &RetainedGeneration,
    verified: &census_report::workbook::publication::VerifiedPublication,
) -> TestResult {
    let request = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: selection.clone(),
    };
    let retry = super::super::super::build(
        store,
        options,
        "report-source-service-after-split",
        &request,
    )?;
    check!(eq; retry.path, current.path);
    check!(eq;
        census_report::workbook::publication::verify_published(Path::new(&retry.path))?.generation_digest,
        verified.generation_digest
    );
    current.preserved()?;
    Ok(())
}

fn build_current(
    store: &census_store::Store,
    options: &crate::workbook::Options,
    selection: &census_domain::model::ProfileSelectionV1,
    performances: &[CanonicalPerformance; 2],
) -> TestResult<(
    String,
    census_report::workbook::publication::VerifiedPublication,
    std::path::PathBuf,
)> {
    let request = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: selection.clone(),
    };
    let path = super::super::super::build(
        store,
        options,
        "report-source-service-after-split",
        &request,
    )?
    .path;
    let verified = replay::verify(Path::new(&path), options.scope, performances)?;
    let generation = Path::new(&path)
        .parent()
        .ok_or("post-split service generation")?
        .to_path_buf();
    Ok((path, verified, generation))
}

pub async fn service_split_acceptance() -> TestResult {
    let (fixture, alias) = Fixture::synthetic_michigan_reviewed().await?;
    let old_public = fixture.public_id.clone();
    let original_candidate = fixture.eligible.athlete.to_string();
    let performances = [fixture.eligible.clone(), fixture.stronger.clone()];
    let (case_id, canonical, prior_ids) = identity_history::merge(&fixture, alias)?;
    check!(eq; canonical.as_str(), old_public);
    let store = &fixture.store;
    let source_root = store.root().to_path_buf();
    let profile = store.snapshot().profile(fixture.selection.profile_id())?;
    let output = super::super::temporary()?;
    let mut options = super::super::super::selected_options(&profile)?;
    options.out = Some(output.path().to_path_buf());
    let prior = build_prior(&fixture, &options)?;
    let prior_verified = replay::verify(Path::new(&prior.path), options.scope, &performances)?;
    assert_aliases(prior.directory()?, &prior_ids)?;
    let successors = split_successors(&fixture, &old_public, &case_id, &canonical)?;
    let (current_path, verified, generation) =
        build_current(store, &options, &fixture.selection, &performances)?;
    check!(ne; verified.generation_digest, prior_verified.generation_digest);
    check!(ne; current_path, prior.path);
    let original_public = successors
        .get(&original_candidate)
        .ok_or("eligible split successor")?;
    super::super::super::check_selected_output(
        Path::new(&current_path),
        original_public,
        &original_candidate,
        &fixture.school_id,
        "14-9",
    )?;
    assert_aliases(&generation, &successors)?;
    let current = RetainedGeneration::capture(current_path)?;
    assert_retry(store, &options, &fixture.selection, &current, &verified)?;
    fixture.remove_sources()?;
    check!(!source_root.exists());
    prior.preserved()?;
    current.preserved()?;
    check!(eq;
        replay::verify(Path::new(&prior.path), options.scope, &performances)?.generation_digest,
        prior_verified.generation_digest
    );
    assert_aliases(prior.directory()?, &prior_ids)?;
    check!(eq;
        replay::verify(Path::new(&current.path), options.scope, &performances)?.generation_digest,
        verified.generation_digest
    );
    assert_aliases(&generation, &successors)?;
    output.close()?;
    Ok(())
}
