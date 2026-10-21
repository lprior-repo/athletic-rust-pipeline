use super::super::{
    ledger::VerifiedLedger,
    material::{self, Material},
    RetainedCaptureLeafV1,
};
use super::*;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentSubjectRoleV1, ReviewedReferenceRecordV1,
};

mod types;
use types::Locator;
pub use types::{
    ActiveAttachmentSubjectDiscoveryV1, ActiveAttachmentSubjectV1, DiscoveredAttachmentCaptureV1,
    DiscoveredAttachmentLocatorV1,
};

pub(in super::super) fn discover<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    accepted: &AcceptedPublicIdentitySnapshot,
    role: AttachmentSubjectRoleV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ActiveAttachmentSubjectDiscoveryV1<'a>> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = discover_checked(ledger, accepted, role, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn discover_checked<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    accepted: &AcceptedPublicIdentitySnapshot,
    role: AttachmentSubjectRoleV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ActiveAttachmentSubjectDiscoveryV1<'a>> {
    let projection = ledger.projection(accepted.store_identity(), grant)?;
    grant
        .charge_steps(ledger.records.len())
        .map_err(Error::Domain)?;
    let mut rows = Vec::new();
    for record in projection.active_records() {
        for subject in record.command.subjects.as_slice() {
            grant.charge_steps(1).map_err(Error::Domain)?;
            if subject.role != role {
                continue;
            }
            let capture = ledger.object(&subject.capture, grant)?;
            let locator = ledger.object(&subject.locator, grant)?;
            let context = ledger.object(&subject.context, grant)?.reference();
            grant.charge_steps(1).map_err(Error::Domain)?;
            let locator = match locator {
                Material::Inline { reference, .. } => {
                    grant
                        .charge_bytes(material::bounded_bytes(reference)?)
                        .map_err(Error::Domain)?;
                    Locator::Inline {
                        reference,
                        bytes: locator.inline_bytes(grant)?,
                    }
                }
                Material::Retained { leaf } => Locator::Retained(leaf),
            };
            let count = rows
                .len()
                .checked_add(1)
                .ok_or(crate::StoreError::CounterOverflow)?;
            if count > 40_000 {
                return Err(Error::Corrupt("active subject discovery metadata bound").into());
            }
            count
                .checked_mul(std::mem::size_of::<ActiveAttachmentSubjectV1<'_>>())
                .ok_or(crate::StoreError::CounterOverflow)?;
            grant
                .charge_bytes(std::mem::size_of::<ActiveAttachmentSubjectV1<'_>>())
                .map_err(Error::Domain)?;
            material::allocate(&mut rows)?;
            let retained_root_anchors = root_anchors(ledger, record, grant)?;
            rows.push(ActiveAttachmentSubjectV1 {
                record,
                subject,
                capture,
                locator,
                context,
                retained_root_anchors,
            });
        }
    }
    Ok(ActiveAttachmentSubjectDiscoveryV1 { rows })
}

fn root_anchors<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    record: &ReviewedReferenceRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<&'a RetainedCaptureLeafV1>> {
    let references = record.command.reviewed_context.objects.as_slice();
    if references.len() > census_domain::participation_candidates::MAX_PARTICIPATION_FACTS {
        return Err(Error::Corrupt("active subject root anchor bound").into());
    }
    let mut anchors = Vec::new();
    for reference in references {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let material = ledger.object(&reference.digest, grant)?;
        if material.reference() != reference {
            return Err(Error::Corrupt("active subject root anchor reference").into());
        }
        let Material::Retained { leaf } = material else {
            continue;
        };
        if leaf.kind != super::super::RetainedCaptureLeafKindV1::RootJson {
            continue;
        }
        grant
            .charge_bytes(std::mem::size_of::<&RetainedCaptureLeafV1>())
            .map_err(Error::Domain)?;
        material::allocate(&mut anchors)?;
        anchors.push(leaf);
    }
    Ok(anchors)
}
