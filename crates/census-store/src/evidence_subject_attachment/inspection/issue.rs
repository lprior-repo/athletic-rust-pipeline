use super::*;
use crate::{StoreError, StoreSnapshot};

pub(in super::super) struct Issued<'s> {
    pub(in super::super) revision: StoreRevision<'s>,
    pub(in super::super) store: StoreIdentity,
    pub(in super::super) head: AttachmentPublicHeadV1,
    pub(in super::super) action: AttachmentActionV1,
    pub(in super::super) target: AttachmentTargetV1,
    pub(in super::super) witness: AttachmentDigestV1,
    pub(in super::super) inputs: Inputs,
    pub(in super::super) objects: Vec<EncodedObject>,
    pub(in super::super) prior_index: Option<usize>,
    pub(in super::super) ledger: Ledger<'s>,
}

pub(super) fn inspect<'s>(
    store: &'s Store,
    request: AttachmentInspectionRequestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Issued<'s>> {
    let guard = AttachmentCommitGuard::acquire(store);
    let view = guard.snapshot();
    let inputs = Inputs::capture(store, &view, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let identity = view.store_identity()?;
    let ledger = Ledger::load(&view, &identity, grant)?;
    grant.charge_steps(2).map_err(Error::Domain)?;
    let head = view.attachment_public_head()?;
    let head = AttachmentPublicHeadV1 {
        sequence: head.sequence,
        digest: AttachmentDigestV1::parse(&head.digest).map_err(Error::Domain)?,
    };
    let (action, selected, witness, objects, prior_index) = match request {
        AttachmentInspectionRequestV1::Revoke { prior } => {
            let (selected, witness, objects, index) = revoked_target(&ledger, &prior, grant)?;
            (
                AttachmentActionV1::Revoke { prior },
                selected,
                witness,
                objects,
                index,
            )
        }
        AttachmentInspectionRequestV1::Attach { target, prior } => {
            let (selected, witness, objects, index) =
                attach(&view, &ledger, &target, &prior, &inputs, grant)?;
            (
                AttachmentActionV1::Attach { prior },
                selected,
                witness,
                objects,
                index,
            )
        }
    };
    Ok(Issued {
        revision: view.revision(),
        store: identity,
        head,
        action,
        target: selected,
        witness,
        inputs,
        objects,
        prior_index,
        ledger,
    })
}

type IssuedTarget = (
    AttachmentTargetV1,
    AttachmentDigestV1,
    Vec<EncodedObject>,
    Option<usize>,
);

fn revoked_target(
    ledger: &Ledger<'_>,
    prior: &AttachmentRecordReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<IssuedTarget> {
    let index = exact_prior(ledger, prior, grant)?;
    let record = ledger
        .records
        .get(index)
        .ok_or(StoreError::CounterOverflow)?;
    Ok((
        record.command.target.clone(),
        record
            .command
            .reviewed_context
            .accepted_target_witness
            .clone(),
        Vec::new(),
        Some(index),
    ))
}

fn attach(
    view: &StoreSnapshot<'_>,
    ledger: &Ledger<'_>,
    id: &PublicAthleteId,
    prior: &AttachmentPredecessorV1,
    inputs: &Inputs,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<IssuedTarget> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let accepted = view
        .accepted_public_identity_snapshot()?
        .ok_or(Error::TargetMoved)?;
    let current = target::current(&accepted, id, grant)?;
    let selected = AttachmentTargetV1 {
        public_id: id.clone(),
        canonical_candidate: current.resolved().representative_candidate.clone(),
        accepted_group_digest: current.group().reference().digest.clone(),
    };
    let index = match prior {
        AttachmentPredecessorV1::First => None,
        AttachmentPredecessorV1::After(prior) => Some(exact_prior(ledger, prior, grant)?),
    };
    if let Some(index) = index {
        let old = ledger
            .records
            .get(index)
            .ok_or(StoreError::CounterOverflow)?;
        if matches!(old.command.action, AttachmentActionV1::Attach { .. }) {
            if selected != old.command.target {
                return Err(Error::TargetMoved.into());
            }
            return Ok((
                selected,
                old.command.reviewed_context.accepted_target_witness.clone(),
                Vec::new(),
                Some(index),
            ));
        }
    }
    let witness = target::encode_witness(
        &current,
        target::WitnessInputs {
            evidence_generation: view.evidence_generation(),
            derived_generation: view.derived_generation(),
            capture: inputs.capture.reference(),
            input_revisions: inputs.token.reference(),
        },
        grant,
    )?;
    let digest = witness.reference().digest.clone();
    let mut objects = Vec::new();
    for object in [current.into_group(), witness] {
        super::super::material::allocate(&mut objects)?;
        objects.push(object);
    }
    Ok((selected, digest, objects, index))
}

fn exact_prior(
    ledger: &Ledger<'_>,
    prior: &AttachmentRecordReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<usize> {
    let index = ledger
        .find_operation(prior.operation.as_str(), grant)?
        .ok_or(Error::Corrupt("inspection predecessor missing"))?;
    if ledger
        .records
        .get(index)
        .is_none_or(|record| record.record_digest != prior.digest)
    {
        return Err(Error::Corrupt("inspection predecessor digest").into());
    }
    Ok(index)
}
