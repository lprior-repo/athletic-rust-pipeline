use super::*;
use census_domain::model::EventSemantics;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn synthetic_legacy_header_is_readable_but_not_current_job_input() -> TestResult {
    historical_header_is_read_only(EventSemantics::Legacy18)
}

#[test]
fn synthetic_flat_events_header_is_readable_but_not_current_job_input() -> TestResult {
    historical_header_is_read_only(EventSemantics::FlatEvents19)
}

fn historical_header_is_read_only(semantics: EventSemantics) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let mut dataset = ExportDataset::load(&store)?;
    check!(eq; dataset.semantic_policy()?, EventSemantics::CURRENT);
    let job = format!("synthetic-policy{}-preflight", semantics.revision());
    dataset.lineage.policy_revision = semantics.revision();
    dataset.lineage.export_job = Some(job.clone());
    bind(&mut dataset)?;
    let directory = store.out_dir().join("export-inputs");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.json", digest(&job)?));
    dataset.save_frozen(&path)?;
    let before = std::fs::read(&path)?;
    let sequence = store.snapshot().sequence();
    let reopened = ExportDataset::reopen_frozen(&path)?;
    check!(eq; reopened.semantic_policy()?, semantics);
    read_only(reopened.ensure_current_policy())?;
    read_only(reopened.ensure_current(&store))?;
    let decoded = super::super::decoded_generations();
    read_only(ExportDataset::for_job(&store, &job))?;
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
    let future = POLICY_REVISION
        .checked_add(1)
        .ok_or("future policy overflow")?;
    for revision in [0, 7, 17, future, u32::MAX] {
        dataset.lineage.policy_revision = revision;
        check!(dataset.semantic_policy().is_err());
        check!(dataset.ensure_current_policy().is_err());
    }
    Ok(())
}

fn read_only<T>(result: Result<T, ReportError>) -> TestResult {
    check!(matches!(
        result,
        Err(ReportError::Invariant { ref detail })
            if detail.contains("historical recruiting policy is read-only")
    ));
    Ok(())
}
