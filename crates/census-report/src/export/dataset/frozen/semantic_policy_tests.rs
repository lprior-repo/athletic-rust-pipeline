use super::*;
use census_domain::model::EventSemantics;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn synthetic_legacy_header_is_readable_but_not_current_job_input() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let mut dataset = ExportDataset::load(&store)?;
    check!(eq; dataset.semantic_policy()?, EventSemantics::FlatEvents19);
    let job = "synthetic-policy18-preflight";
    dataset.lineage.policy_revision = 18;
    dataset.lineage.export_job = Some(job.into());
    bind(&mut dataset)?;
    let directory = store.out_dir().join("export-inputs");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.json", digest(&job)?));
    dataset.save_frozen(&path)?;
    let before = std::fs::read(&path)?;
    let sequence = store.snapshot().sequence();
    let reopened = ExportDataset::reopen_frozen(&path)?;
    check!(eq; reopened.semantic_policy()?, EventSemantics::Legacy18);
    check!(reopened.ensure_current_policy().is_err());
    check!(reopened.ensure_current(&store).is_err());
    let decoded = super::super::decoded_generations();
    check!(ExportDataset::for_job(&store, job).is_err());
    check!(eq; super::super::decoded_generations(), decoded);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; std::fs::read(&path)?, before);
    Ok(())
}

#[test]
fn unsupported_semantic_revisions_remain_rejected() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let mut dataset = ExportDataset::load(&store)?;
    for revision in [0, 7, 17, 20, u32::MAX] {
        dataset.lineage.policy_revision = revision;
        check!(dataset.semantic_policy().is_err());
        check!(dataset.ensure_current_policy().is_err());
    }
    Ok(())
}
