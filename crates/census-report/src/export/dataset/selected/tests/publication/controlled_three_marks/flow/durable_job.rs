use super::*;
use crate::export::ExportJobRequest;
use census_domain::model::ProfileSelectionV1;

#[test]
fn selected_durable_job_preserves_best_school_public_id_retry_and_revocation() -> TestResult {
    let installed = source::install()?;
    let (current, accepted50) = reviewed_inputs(&installed)?;
    let mut input = super::super::selection::profile()?.input().clone();
    input.jurisdictions = census_domain::UsJurisdiction::CENSUS_SCOPE.to_vec();
    let profile = QueryProfileV1::new(input)?;
    installed.store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(installed.store.store_identity()?, profile.id()?);
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: selection.clone(),
    };
    let job = "controlled-selected-national-workbook-v1";
    let output = tempfile::tempdir()?;
    let dataset = ExportDataset::for_job_request(&installed.store, job, &request)?;
    let root = output.path().join("publication");
    let workbook = super::super::publish::publish(&dataset, &installed.store, &profile, &root)?;
    assert_output(
        &dataset,
        &workbook,
        &installed.public,
        &current,
        &installed.school,
        Generation::Initial,
    )?;
    let input_generation = dataset.lineage.input_generation.clone();
    let frozen_digest = dataset.archive_digest()?;
    let generation = publication::verify_published(&workbook)?.generation_digest;
    let bytes = std::fs::read(&workbook)?;
    drop(dataset);
    installed
        .store
        .journal_done("selected-job-progress", "unrelated", &true)?;
    let retry = ExportDataset::for_job_request(&installed.store, job, &request)?;
    check!(eq; retry.lineage.input_generation, input_generation);
    check!(eq; retry.archive_digest()?, frozen_digest);
    let repeated = super::super::publish::publish(&retry, &installed.store, &profile, &root)?;
    check!(eq; repeated, workbook);
    check!(eq; publication::verify_published(&repeated)?.generation_digest, generation);
    let other_profile = super::super::selection::profile()?;
    installed.store.register_profile(&other_profile)?;
    let mismatch = ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(installed.store.store_identity()?, other_profile.id()?),
    };
    check!(ExportDataset::for_job_request(&installed.store, job, &mismatch).is_err());
    super::super::documentary_lifecycle::revoke(&installed, &accepted50)?;
    require_stale_recovery_preserved(&installed.store, job, &request, &root, &workbook)?;
    check!(ExportDataset::for_job_request(&installed.store, job, &request).is_err());
    check!(super::super::publish::publish(&retry, &installed.store, &profile, &root).is_err());
    check!(eq; std::fs::read(&workbook)?, bytes);
    check!(eq; publication::verify_published(&workbook)?.generation_digest, generation);
    let corrected = ExportDataset::for_job_request(
        &installed.store,
        "controlled-selected-corrected-v1",
        &request,
    )?;
    let corrected_path = super::super::publish::publish(
        &corrected,
        &installed.store,
        &profile,
        &output.path().join("corrected"),
    )?;
    assert_output(
        &corrected,
        &corrected_path,
        &installed.public,
        &current,
        &installed.school,
        Generation::StatusWithdrawn,
    )?;
    let archive = workbook
        .parent()
        .ok_or("workbook generation")?
        .join("frozen-input.json");
    let reopened = ExportDataset::reopen_frozen_with_digest(&archive, &frozen_digest)?;
    assert_output(
        &reopened,
        &workbook,
        &installed.public,
        &current,
        &installed.school,
        Generation::Initial,
    )?;
    check!(super::super::publish::publish(&reopened, &installed.store, &profile, &root).is_err());
    Ok(())
}

#[test]
fn selected_job_refuses_documentary_withdrawal_immediately_before_final_pointer_fence() -> TestResult
{
    let installed = source::install()?;
    let (current, accepted50) = reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    installed.store.register_profile(&profile)?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(installed.store.store_identity()?, profile.id()?),
    };
    let initial =
        ExportDataset::for_job_request(&installed.store, "selected-final-fence-g0", &request)?;
    let output = tempfile::tempdir()?;
    let root = output.path().join("publication");
    let workbook = super::super::publish::publish(&initial, &installed.store, &profile, &root)?;
    let original = publication::verify_published(&workbook)?.generation_digest;
    let pointer = std::fs::read_link(root.join("current"))?;
    drop(initial);
    let next =
        ExportDataset::for_job_request(&installed.store, "selected-final-fence-g1", &request)?;
    let options = crate::workbook::Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: Some(root.clone()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: profile.input().jurisdictions.clone(),
        school_year: profile.input().contact_school_year,
    };
    let censuses =
        crate::workbook::Censuses::of(&next, &installed.store.out_dir(), &options.jurisdictions)?;
    let callback = || {
        super::super::documentary_lifecycle::revoke(&installed, &accepted50)
            .map(|_| ())
            .map_err(|error| crate::report::ReportError::Invariant {
                detail: error.to_string(),
            })
    };
    let outcome = publication::build_before_input_fence(
        &next,
        &installed.store,
        &options,
        &censuses,
        &callback,
    );
    check!(
        matches!(outcome, Err(ref error) if error.to_string().contains("selected export job authority changed"))
    );
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; publication::verify_published(&workbook)?.generation_digest, original);
    check!(current != installed.school);
    Ok(())
}

#[test]
fn selected_job_interrupted_intent_requires_verified_current_input_for_recovery() -> TestResult {
    let installed = source::install()?;
    let (_current, accepted50) = reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    installed.store.register_profile(&profile)?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(installed.store.store_identity()?, profile.id()?),
    };
    let initial =
        ExportDataset::for_job_request(&installed.store, "selected-recovery-g0", &request)?;
    let output = tempfile::tempdir()?;
    let root = output.path().join("publication");
    let workbook = super::super::publish::publish(&initial, &installed.store, &profile, &root)?;
    let original = publication::verify_published(&workbook)?.generation_digest;
    let pointer = std::fs::read_link(root.join("current"))?;
    drop(initial);
    let next = ExportDataset::for_job_request(&installed.store, "selected-recovery-g1", &request)?;
    let options = crate::workbook::Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: Some(root.clone()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: profile.input().jurisdictions.clone(),
        school_year: profile.input().contact_school_year,
    };
    let censuses =
        crate::workbook::Censuses::of(&next, &installed.store.out_dir(), &options.jurisdictions)?;
    let interrupted = || {
        Err(crate::report::ReportError::Invariant {
            detail: "controlled after-intent interruption".into(),
        })
    };
    check!(publication::build_after_input_intent(
        &next,
        &installed.store,
        &options,
        &censuses,
        &interrupted
    )
    .is_err());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(
        matches!(publication::recover_publication(&installed.store, &root), Err(ref error) if error.to_string().contains("selected job recovery requires its verified durable job input"))
    );
    let recovered = super::super::publish::publish(&next, &installed.store, &profile, &root)?;
    check!(ne; publication::verify_published(&recovered)?.generation_digest, original);
    let recovered_pointer = std::fs::read_link(root.join("current"))?;
    drop(next);
    let stale = ExportDataset::for_job_request(&installed.store, "selected-recovery-g2", &request)?;
    let censuses =
        crate::workbook::Censuses::of(&stale, &installed.store.out_dir(), &options.jurisdictions)?;
    check!(publication::build_after_input_intent(
        &stale,
        &installed.store,
        &options,
        &censuses,
        &interrupted
    )
    .is_err());
    super::super::documentary_lifecycle::revoke(&installed, &accepted50)?;
    check!(publication::recover_publication(&installed.store, &root).is_err());
    check!(super::super::publish::publish(&stale, &installed.store, &profile, &root).is_err());
    check!(eq; std::fs::read_link(root.join("current"))?, recovered_pointer);
    check!(eq; publication::verify_published(&workbook)?.generation_digest, original);
    Ok(())
}

fn require_stale_recovery_preserved(
    store: &census_store::Store,
    job: &str,
    request: &ExportJobRequest,
    publication_root: &std::path::Path,
    workbook: &std::path::Path,
) -> TestResult {
    let input_root = store.out_dir().join("export-inputs").join(format!(
        "{}.selected-v1",
        census_domain::model::serialized_digest(&job)?
    ));
    let archive = std::fs::read(input_root.join("input.json"))?;
    let commitment = std::fs::read(input_root.join("commitment.json"))?;
    let pointer = std::fs::read_link(publication_root.join("current"))?;
    let manifest_path = workbook
        .parent()
        .ok_or("published generation")?
        .join("manifest.json");
    let manifest = std::fs::read(&manifest_path)?;
    let bytes = std::fs::read(workbook)?;
    let sequence = store.snapshot().sequence();
    let error = ExportDataset::recover_job_request(store, job, request)
        .err()
        .ok_or("stale committed recovery unexpectedly accepted")?;
    check!(
        matches!(error, crate::report::ReportError::Invariant { ref detail }
        if detail == "selected export job authority changed; retained input is preserved")
    );
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; std::fs::read(input_root.join("input.json"))?, archive);
    check!(eq; std::fs::read(input_root.join("commitment.json"))?, commitment);
    check!(eq; std::fs::read_link(publication_root.join("current"))?, pointer);
    check!(eq; std::fs::read(&manifest_path)?, manifest);
    check!(eq; std::fs::read(workbook)?, bytes);
    Ok(())
}
