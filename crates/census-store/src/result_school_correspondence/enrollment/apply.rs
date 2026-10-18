use super::{grant_error, guarded, Error, PreparedResultSchoolEnrollmentV1};
use crate::{Application, Receipt, Store, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_school_correspondence::*;
use chrono::Utc;
use fjall::PersistMode;

pub(super) fn apply(
    store: &Store,
    prepared: PreparedResultSchoolEnrollmentV1<'_>,
    approved: &[u8],
    approval_digest: &ResultSchoolDigestV1,
) -> StoreResult<Application> {
    let PreparedResultSchoolEnrollmentV1 {
        revision,
        command,
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
            return super::replay::require_retained(store, &command, &expected, &receipt, grant)
                .map(Application::Repeated);
        }

        super::prepare::require_current(store, &revision, &command, grant)?;
        commit(store, command, expected, grant)
    })
}

fn commit(
    store: &Store,
    command: ResultSchoolEnrollmentCommandV1,
    approval_digest: ResultSchoolDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    let committed_at = Utc::now();
    if committed_at < command.reviewed_at {
        return Err(Error::Corrupt("commit precedes review").into());
    }

    let initial_inventory = super::super::wire::initial(&command, grant)?;
    let head = super::super::codec::encode(
        &super::super::wire::AuthorityHead {
            schema: "result-school-authority-head-v1".into(),
            inventory: initial_inventory.clone(),
        },
        grant,
    )?;

    let operation = super::super::wire::enrollment_operation(&command);
    let enrollment = super::super::codec::encode(
        &super::super::wire::Enrollment {
            schema: "result-school-enrollment-v1".into(),
            command,
            initial_inventory,
            approval_digest,
            committed_at,
        },
        grant,
    )?;

    let receipt = Receipt {
        operation,
        digest: super::super::codec::digest(&enrollment, grant)?
            .as_str()
            .to_owned(),
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
        "9",
    );
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;

    Ok(Application::Written(receipt))
}
