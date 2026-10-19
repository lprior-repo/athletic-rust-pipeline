use super::{grant_error, guarded, Error, PreparedResultCalendarEnrollmentV1};
use crate::{Application, Receipt, Store, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_calendar_correspondence::*;
use chrono::Utc;
use fjall::PersistMode;

pub(super) fn apply(
    store: &Store,
    prepared: PreparedResultCalendarEnrollmentV1<'_>,
    approved: &[u8],
    approval_digest: &ResultCalendarDigestV1,
) -> StoreResult<Application> {
    let PreparedResultCalendarEnrollmentV1 {
        revision,
        command,
        prior_school,
        approval,
        approval_digest: expected,
        replay,
        mut grant,
    } = prepared;
    guarded(&mut grant, |grant| {
        revision.ensure_origin(store, command.operation.as_str())?;
        grant.charge_bytes(approved.len()).map_err(grant_error)?;
        grant.charge_steps(1).map_err(grant_error)?;
        if approved != approval || approval_digest != &expected {
            return Err(Error::ApprovalMismatch.into());
        }
        if let Some(receipt) = replay {
            return super::replay::require_retained(
                store,
                &command,
                &prior_school,
                &expected,
                &receipt,
                grant,
            )
            .map(Application::Repeated);
        }
        super::prepare::require_current(
            store,
            &revision,
            &command,
            prior_school.get().as_bytes(),
            grant,
        )?;
        commit(store, command, prior_school, expected, grant)
    })
}

fn commit(
    store: &Store,
    command: ResultCalendarEnrollmentCommandV1,
    prior_school: Box<serde_json::value::RawValue>,
    approval_digest: ResultCalendarDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    let committed_at = commit_time(&command.reviewed_at)?;
    let initial_inventory = super::super::wire::initial(&command, grant)?;
    let head = super::super::codec::encode(
        &super::super::wire::AuthorityHead {
            schema: "result-calendar-authority-head-v1".into(),
            inventory: initial_inventory.clone(),
        },
        grant,
    )?;
    let operation = super::super::wire::enrollment_operation(&command);
    let enrollment = super::super::codec::encode(
        &super::super::wire::Enrollment {
            schema: "result-calendar-enrollment-v1".into(),
            command,
            prior_school,
            initial_inventory,
            approval_digest,
            committed_at,
        },
        grant,
    )?;
    let digest = super::super::codec::digest(&enrollment, grant)?;
    let receipt = Receipt {
        operation,
        digest: digest.as_str().to_owned(),
        at: committed_at.to_rfc3339(),
        appended: 0,
        credited: false,
    };
    let encoded_receipt = super::super::codec::encode(&receipt, grant)?;
    grant.charge_steps(4).map_err(grant_error)?;
    let mut batch = store.db.batch();
    batch.insert(&store.meta, super::super::ENROLLMENT, enrollment);
    batch.insert(&store.meta, super::super::wire::HEAD, head);
    batch.insert(
        &store.receipts,
        receipt.operation.as_bytes(),
        encoded_receipt,
    );
    crate::meta::put_text(
        &mut batch,
        &store.meta,
        crate::format::LIFECYCLE_REVISION_KEY,
        "10",
    );
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(Application::Written(receipt))
}

fn commit_time(reviewed_at: &chrono::DateTime<Utc>) -> StoreResult<chrono::DateTime<Utc>> {
    let committed_at = Utc::now();
    if committed_at < *reviewed_at {
        return Err(Error::Corrupt("commit precedes review").into());
    }
    Ok(committed_at)
}
