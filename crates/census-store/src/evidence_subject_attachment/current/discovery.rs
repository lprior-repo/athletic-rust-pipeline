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
    ActiveResultSubjectDiscoveryV1, ActiveResultSubjectV1, DiscoveredAttachmentCaptureV1,
    DiscoveredAttachmentLocatorV1,
};

pub(in super::super) fn discover<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    accepted: &AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ActiveResultSubjectDiscoveryV1<'a>> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = discover_checked(ledger, accepted, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn discover_checked<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    accepted: &AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ActiveResultSubjectDiscoveryV1<'a>> {
    let projection = ledger.projection(accepted.store_identity(), grant)?;
    grant
        .charge_steps(ledger.records.len())
        .map_err(Error::Domain)?;
    let mut rows = Vec::new();
    for record in projection.active_records() {
        for subject in record.command.subjects.as_slice() {
            grant.charge_steps(1).map_err(Error::Domain)?;
            if subject.role != AttachmentSubjectRoleV1::ResultRow {
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
                .checked_mul(std::mem::size_of::<ActiveResultSubjectV1<'_>>())
                .ok_or(crate::StoreError::CounterOverflow)?;
            material::allocate(&mut rows)?;
            rows.push(ActiveResultSubjectV1 {
                record,
                subject,
                capture,
                locator,
                context,
            });
        }
    }
    Ok(ActiveResultSubjectDiscoveryV1 { rows })
}
