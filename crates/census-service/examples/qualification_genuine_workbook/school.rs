use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, read_recorded_provider_school,
};
use census_crawl::participation_evidence::result_ownership::school_correspondence;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::SchoolId;
use census_domain::result_school_correspondence::{
    ResultSchoolActionV1, ResultSchoolDigestV1, ResultSchoolPredecessorV1,
    ResultSchoolRecordReferenceV1, ResultSchoolReviewActorV1, ResultSchoolReviewCommandV1,
    ResultSchoolReviewedRecordV1, ResultSchoolTextV1, RESULT_SCHOOL_POLICY_V1,
};
use census_store::{Application, ResultSchoolCorrespondencePreparationV1, Store};
use std::path::Path;

const ORIGINAL_OPERATION: &str = "genuine-school-accept-may9";
const EVENT_CONTEXT_OPERATION: &str = "genuine-school-accept-checked-event-context-may9";

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > limit {
        return Err(format!("retained school artifact exceeds bound: {}", path.display()).into());
    }
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(usize::try_from(metadata.len())?)?;
    bytes.resize(usize::try_from(metadata.len())?, 0);
    std::io::Read::read_exact(&mut file, &mut bytes)?;
    Ok(bytes)
}
fn retain(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            std::io::Write::write_all(&mut file, bytes)?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let held = read_bounded(path, u64::try_from(bytes.len())?)?;
            if held != bytes {
                return Err(
                    format!("refusing to replace immutable artifact: {}", path.display()).into(),
                );
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn record_reference(record: &ResultSchoolReviewedRecordV1) -> ResultSchoolRecordReferenceV1 {
    ResultSchoolRecordReferenceV1 {
        operation: record.command.operation.clone(),
        digest: record.record_digest.clone(),
    }
}

fn apply_original_replay(
    store: &Store,
    record: &ResultSchoolReviewedRecordV1,
    objects: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let command_bytes = read_bounded(&objects.join("genuine-school-command.json"), 256 * 1024)?;
    let command: ResultSchoolReviewCommandV1 = serde_json::from_slice(&command_bytes)?;
    if command != record.command {
        return Err("retained original school command differs from historical record".into());
    }
    let approval = read_bounded(&objects.join("genuine-school-approval.bin"), 1024 * 1024)?;
    let prepared = store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::ReplayOnly,
    )?;
    if !prepared.is_replay() || prepared.approval_bytes() != approval {
        return Err("original school replay preparation differs from retained approval".into());
    }
    if prepared.approval_digest() != &record.approval_digest {
        return Err("original school replay digest differs from historical record".into());
    }
    let result = store.apply_result_school_correspondence_v1(
        prepared,
        &approval,
        &record.approval_digest,
    )?;
    let receipt = match result {
        Application::Repeated(receipt) => receipt,
        Application::Written(_) => return Err("original school replay wrote a new record".into()),
    };
    let expected = store
        .receipt(&receipt.operation)?
        .ok_or("original school receipt absent")?;
    if receipt != expected {
        return Err("original school replay receipt differs from store receipt".into());
    }
    retain(
        &objects.join("genuine-school-original-replayreceipt.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(())
}

fn apply_event_replay(
    store: &Store,
    record: &ResultSchoolReviewedRecordV1,
    objects: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let prefix = "distinct-school-event-context";
    let approval = read_bounded(&objects.join(format!("{prefix}-approval.bin")), 1024 * 1024)?;
    let digest = ResultSchoolDigestV1::parse(std::str::from_utf8(&read_bounded(
        &objects.join(format!("{prefix}-approval.digest")),
        256,
    )?)?)?;
    if digest != record.approval_digest {
        return Err("event-context replay digest differs from retained record".into());
    }
    let command_bytes = read_bounded(&objects.join(format!("{prefix}-command.json")), 256 * 1024)?;
    let command: ResultSchoolReviewCommandV1 = serde_json::from_slice(&command_bytes)?;
    if command != record.command {
        return Err("event-context replay command differs from retained record".into());
    }
    let prepared = store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::ReplayOnly,
    )?;
    if !prepared.is_replay()
        || prepared.approval_bytes() != approval
        || prepared.approval_digest() != &digest
    {
        return Err("event-context replay preparation differs from retained approval".into());
    }
    let result = store.apply_result_school_correspondence_v1(prepared, &approval, &digest)?;
    let receipt = match result {
        Application::Repeated(receipt) => receipt,
        Application::Written(_) => return Err("event-context replay wrote a new record".into()),
    };
    let retained = read_bounded(&objects.join(format!("{prefix}-receipt.json")), 256 * 1024)?;
    let expected: census_store::Receipt = serde_json::from_slice(&retained)?;
    if receipt != expected || store.receipt(&receipt.operation)?.as_ref() != Some(&receipt) {
        return Err("event-context replay receipt differs from retained receipt".into());
    }
    Ok(())
}

pub(super) fn ensure(
    store: &Store,
    root: &Path,
    objects: &Path,
    context: &str,
    locator: SourceRowLocator,
) -> Result<(SchoolId, String), Box<dyn std::error::Error>> {
    let snapshot = store.snapshot();
    let mut source_grant = AttachmentReplayGrantV1::new();
    let reference = super::provider_school_reference(root)?;
    let provider = read_recorded_provider_school(&snapshot, &reference, &mut source_grant)?
        .ok_or("pinned provider school packet absent in store")?;
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut source_grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut source_grant)?;
    let discovered = discover_current_html_results(&attachments, &mut source_grant)?;
    let proposal = (0..discovered.group_count())
        .try_fold(
            None,
            |found, index| -> Result<_, Box<dyn std::error::Error>> {
                if found.is_some() {
                    return Ok(found);
                }
                let candidate =
                    discovered.with_group(index, &mut source_grant, |linked, grant| {
                        if linked.recorded_root().key.as_str() != context || linked.row_count() != 1
                        {
                            return Ok(None);
                        }
                        let row = linked.row(0).ok_or(census_store::StoreError::Refused {
                            detail: "school result group row absent".into(),
                        })?;
                        if row.checked_row().locator() != locator {
                            return Err(census_store::StoreError::Refused {
                                detail: "school result row locator differs".into(),
                            }
                            .into());
                        }
                        let scope =
                            super::school_scope(linked.checked_document(), row.checked_row())?;
                        school_correspondence::build_claim(
                            &attachments,
                            &linked,
                            0,
                            &provider,
                            scope,
                            grant,
                        )
                        .map(Some)
                    })?;
                Ok(candidate)
            },
        )?
        .ok_or("school correspondence missing for current event-context row")?;
    let historical = current
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == ORIGINAL_OPERATION);
    if let Some(record) = historical {
        apply_original_replay(store, record, objects)?;
        if proposal.claim().statement != record.command.source.statement
            || proposal.claim().target != record.command.source.target
        {
            return Err("current event-context source statement or target differs from original school record".into());
        }
    } else {
        let inspection =
            store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
        let command = ResultSchoolReviewCommandV1 {
            version: 1,
            policy: ResultSchoolTextV1::parse(RESULT_SCHOOL_POLICY_V1)?,
            store: inspection.store_identity().clone(),
            operation: ResultSchoolTextV1::parse(ORIGINAL_OPERATION)?,
            input_token: inspection.input_token().clone(),
            evidence_generation: inspection.evidence_generation(),
            derived_generation: inspection.derived_generation(),
            accepted_head: inspection.public_head().clone(),
            expected_inventory: inspection.inventory().clone(),
            source: proposal.claim().clone(),
            action: ResultSchoolActionV1::Accept {
                prior: ResultSchoolPredecessorV1::First,
            },
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: chrono::Utc::now(),
            rationale: ResultSchoolTextV1::parse(
                "Genuine original school-cell correspondence review on the exact performance day",
            )?,
        };
        let context_objects = proposal.borrowed_context(&mut source_grant)?;
        retain(
            &objects.join("genuine-school-command.json"),
            &serde_json::to_vec_pretty(&command)?,
        )?;
        let prepared = store.prepare_result_school_correspondence_v1(
            command,
            ResultSchoolCorrespondencePreparationV1::New {
                inspection,
                context: &context_objects,
            },
        )?;
        let approval = prepared.approval_bytes().to_vec();
        let digest = prepared.approval_digest().clone();
        retain(&objects.join("genuine-school-approval.bin"), &approval)?;
        match store.apply_result_school_correspondence_v1(prepared, &approval, &digest)? {
            Application::Written(_) | Application::Repeated(_) => {}
        }
    }
    let refreshed = store.snapshot();
    let mut refreshed_grant = AttachmentReplayGrantV1::new();
    let attachments =
        refreshed.read_current_evidence_subject_attachment_references_v1(&mut refreshed_grant)?;
    let current_school = refreshed
        .read_current_result_school_correspondences_v1(&attachments, &mut refreshed_grant)?;
    let original = current_school
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == ORIGINAL_OPERATION)
        .ok_or("original school record absent after review")?;
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = ResultSchoolReviewCommandV1 {
        version: 1,
        policy: ResultSchoolTextV1::parse(RESULT_SCHOOL_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: ResultSchoolTextV1::parse(EVENT_CONTEXT_OPERATION)?,
        input_token: inspection.input_token().clone(),
        evidence_generation: inspection.evidence_generation(),
        derived_generation: inspection.derived_generation(),
        accepted_head: inspection.public_head().clone(),
        expected_inventory: inspection.inventory().clone(),
        source: proposal.claim().clone(),
        action: ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::After(record_reference(original)),
        },
        actor: ResultSchoolReviewActorV1::LocalOperator,
        reviewed_at: chrono::Utc::now(),
        rationale: ResultSchoolTextV1::parse(
            "Genuine checked event-context school correspondence review",
        )?,
    };
    let existing = current_school
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == EVENT_CONTEXT_OPERATION);
    if let Some(record) = existing {
        if record.command.source != command.source || record.command.action != command.action {
            return Err("existing event-context school record differs from current source".into());
        }
        apply_event_replay(store, record, objects)?;
    } else {
        let context_objects = proposal.borrowed_context(&mut source_grant)?;
        retain(
            &objects.join("distinct-school-event-context-command.json"),
            &serde_json::to_vec_pretty(&command)?,
        )?;
        let prepared = store.prepare_result_school_correspondence_v1(
            command,
            ResultSchoolCorrespondencePreparationV1::New {
                inspection,
                context: &context_objects,
            },
        )?;
        let approval = prepared.approval_bytes().to_vec();
        let digest = prepared.approval_digest().clone();
        retain(
            &objects.join("distinct-school-event-context-approval.bin"),
            &approval,
        )?;
        retain(
            &objects.join("distinct-school-event-context-approval.digest"),
            digest.as_str().as_bytes(),
        )?;
        let receipt =
            match store.apply_result_school_correspondence_v1(prepared, &approval, &digest)? {
                Application::Written(receipt) | Application::Repeated(receipt) => receipt,
            };
        if store.receipt(&receipt.operation)?.as_ref() != Some(&receipt) {
            return Err("event-context school receipt differs from durable receipt".into());
        }
        retain(
            &objects.join("distinct-school-event-context-receipt.json"),
            &serde_json::to_vec_pretty(&receipt)?,
        )?;
    }
    let final_snapshot = store.snapshot();
    let mut final_grant = AttachmentReplayGrantV1::new();
    let final_attachments =
        final_snapshot.read_current_evidence_subject_attachment_references_v1(&mut final_grant)?;
    let final_school = final_snapshot
        .read_current_result_school_correspondences_v1(&final_attachments, &mut final_grant)?;
    let final_record = final_school
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == EVENT_CONTEXT_OPERATION)
        .ok_or("event-context school record absent")?;
    retain(
        &objects.join("distinct-school-event-context-record.json"),
        &serde_json::to_vec_pretty(final_record)?,
    )?;
    Ok((
        final_record.command.source.target.school.clone(),
        final_record.record_digest.as_str().to_owned(),
    ))
}
