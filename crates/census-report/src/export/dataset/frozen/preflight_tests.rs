use super::super::{decoded_generations, ExportDataset};
use crate::report::ReportError;
use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn historical_publication_policy_is_refused(revision: u32) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let current = ExportDataset::load(&store)?;
    check!(eq; current.lineage.policy_revision, 18);
    let archive = temporary.path().join(format!("revision-{revision}.json"));
    current.save_frozen(&archive)?;
    let mut historical: serde_json::Value = serde_json::from_slice(&std::fs::read(&archive)?)?;
    let mut old_lineage = current.lineage.clone();
    old_lineage.policy_revision = revision;
    old_lineage.input_generation = super::generation(&old_lineage)?;
    check!(ne; old_lineage.input_generation, current.lineage.input_generation);
    historical["lineage"] = serde_json::to_value(old_lineage)?;
    let retained = serde_json::to_vec(&historical)?;
    std::fs::write(&archive, &retained)?;
    let error = match ExportDataset::reopen_frozen(&archive) {
        Err(error) => error,
        Ok(_) => return Err("old conflict policy was silently reinterpreted".into()),
    };
    check!(
        matches!(error, ReportError::Invariant { ref detail } if detail == "unsupported or malformed frozen export lineage")
    );
    check!(eq; std::fs::read(&archive)?, retained);
    Ok(())
}

#[test]
fn stale_job_is_refused_before_a_complete_generation_is_decoded() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let job = "preflight-stale-job";
    let captured = ExportDataset::for_job(&store, job)?;
    let lineage = captured.lineage.clone();
    drop(captured);
    let archive = store.out_dir().join("export-inputs").join(format!(
        "{}.json",
        census_domain::model::serialized_digest(&job)?
    ));
    let retained_bytes = std::fs::read(&archive)?;
    let (school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Later School", "later", None);
    store.append(Table::Schools, &school)?;
    let decoded = decoded_generations();
    let error = match ExportDataset::for_job(&store, job) {
        Err(error) => error,
        Ok(_) => return Err("stale job decoded and reopened".into()),
    };
    check!(
        matches!(error, ReportError::Invariant { ref detail } if detail.contains("stale or foreign"))
    );
    check!(eq; decoded_generations(), decoded);
    check!(eq; std::fs::read(&archive)?, retained_bytes);
    check!(eq; ExportDataset::reopen_frozen(&archive)?.lineage, lineage);
    Ok(())
}

#[test]
fn unsupported_lineage_is_refused_before_malformed_table_rows_are_decoded() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let archive = temporary.path().join("frozen.json");
    ExportDataset::load(&store)?.save_frozen(&archive)?;
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&archive)?)?;
    value["lineage"]["policy_revision"] = serde_json::json!(0);
    value["data"]["athletes"] = serde_json::json!(["not an athlete"]);
    let retained = serde_json::to_vec(&value)?;
    std::fs::write(&archive, &retained)?;
    let error = match ExportDataset::reopen_frozen(&archive) {
        Err(error) => error,
        Ok(_) => return Err("unsupported archive reopened".into()),
    };
    check!(
        matches!(error, ReportError::Invariant { ref detail } if detail == "unsupported or malformed frozen export lineage")
    );
    check!(eq; std::fs::read(&archive)?, retained);
    Ok(())
}

#[test]
fn retained_conflict_policy_preserves_but_refuses_revision_seven_inputs() -> TestResult {
    historical_publication_policy_is_refused(7)
}

#[test]
fn tie_policy_preserves_but_refuses_revision_eight_inputs() -> TestResult {
    historical_publication_policy_is_refused(8)
}

#[test]
fn policy_nine_archive_is_refused_without_rewriting_retained_bytes() -> TestResult {
    historical_publication_policy_is_refused(9)
}

#[test]
fn policy_ten_archive_is_refused_without_rewriting_retained_bytes() -> TestResult {
    historical_publication_policy_is_refused(10)
}

#[test]
fn pushed_policy_twelve_archive_is_refused_without_rewriting_retained_bytes() -> TestResult {
    historical_publication_policy_is_refused(12)
}

#[test]
fn policy_fifteen_archive_is_refused_without_rewriting_retained_bytes() -> TestResult {
    historical_publication_policy_is_refused(15)
}

#[test]
fn policy_sixteen_archive_is_refused_without_rewriting_retained_bytes() -> TestResult {
    historical_publication_policy_is_refused(16)
}
