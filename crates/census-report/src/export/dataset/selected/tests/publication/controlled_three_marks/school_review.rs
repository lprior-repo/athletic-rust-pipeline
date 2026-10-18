use super::source::{self, InstalledSource, TestResult};
use census_crawl::participation_evidence::recorded::discover_current_html_results;
use census_crawl::participation_evidence::result_ownership::school_correspondence::read::checked_current;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::SchoolId;
use census_store::{Store, StoreError};
use std::collections::BTreeMap;

type Observations = BTreeMap<String, (SchoolId, String, String)>;

fn observe(store: &Store) -> TestResult<Observations> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let schools =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let discovered = discover_current_html_results(&attachments, &mut grant)?;
    let mut observations = BTreeMap::new();
    for index in 0..discovered.group_count() {
        let (key, school, day, digest, locator, original_locator) =
            discovered.with_group(index, &mut grant, |linked, grant| {
                let original = linked.row(0).ok_or_else(|| StoreError::Refused {
                    detail: "controlled original row missing".into(),
                })?;
                let checked = checked_current(&schools, &attachments, &linked, 0, grant)?
                    .ok_or_else(|| StoreError::Refused {
                        detail: "controlled historical school authority missing".into(),
                    })?;
                let record = checked.record();
                Ok((
                    linked.recorded_root().key.clone(),
                    checked.school().clone(),
                    record.command.source.target.day.as_str().to_owned(),
                    record.record_digest.as_str().to_owned(),
                    record.command.source.statement.row_locator.clone(),
                    original.attachment().subject().locator.clone(),
                ))
            })?;
        check!(eq; locator, original_locator);
        if observations.insert(key, (school, day, digest)).is_some() {
            return Err("duplicate controlled original document".into());
        }
    }
    Ok(observations)
}

#[test]
fn controlled_original_school_review_preserves_each_performance_day_after_cold_reopen() -> TestResult
{
    let installed = source::install()?;
    let points = super::point_source::install(&installed)?;
    let target_status = points.get(1).ok_or("controlled May15 point missing")?;
    super::school_authority::establish(&installed, target_status)?;
    let before = observe(&installed.store)?;
    for (locator, mark) in installed.row_locators.iter().zip(source::MARKS) {
        let (school, day, _) = before
            .get(&locator.document_key)
            .ok_or("school correspondence missing for original document")?;
        check!(eq; school, &installed.school);
        check!(eq; day, mark.date);
    }
    source::require_originals(&installed.store, &installed.source_records)?;
    let InstalledSource {
        directory,
        store,
        candidate,
        public,
        source_records,
        ..
    } = installed;
    drop(store);
    let reopened = source::reopen(&directory)?;
    source::require_identity(&reopened, &candidate, &public)?;
    source::require_originals(&reopened, &source_records)?;
    check!(eq; observe(&reopened)?, before);
    Ok(())
}
