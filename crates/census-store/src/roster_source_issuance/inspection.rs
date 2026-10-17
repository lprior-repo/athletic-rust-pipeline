use super::{codec, format, RosterSourceIssuanceStoreError as Error};
use super::{RosterStartupValidationBudgetV1, HEAD, LIFECYCLE};
use crate::{RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{SchoolId, StoreIdentity, TeamId};
use fjall::Readable;

mod evidence;
mod admission;
mod inventory;
mod manifest;
pub use evidence::RosterSourceEvidenceV1;
pub use inventory::RosterPhysicalProjectionRowV1;

pub struct RosterIssuanceInspectionRequestV1 {
    pub root: RetainedCaptureRootV1,
    pub school: SchoolId,
    pub program: TeamId,
}

pub struct RosterIssuanceInspectionV1<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) store: StoreIdentity,
    pub(super) input_token: Vec<u8>,
    pub(super) authority_head: Vec<u8>,
    pub(super) request: RosterIssuanceInspectionRequestV1,
    pub(super) evidence: RosterSourceEvidenceV1,
    pub(super) grant: AttachmentReplayGrantV1,
}

impl RosterIssuanceInspectionV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity { &self.store }
    pub fn evidence_generation(&self) -> u64 { self.revision.evidence_generation() }
    pub fn derived_generation(&self) -> u64 { self.revision.derived_generation() }
    pub fn input_token_digest(&self) -> String { codec::digest(&self.input_token) }
    pub fn authority_head_digest(&self) -> String { codec::digest(&self.authority_head) }
    pub fn requested_school(&self) -> &SchoolId { &self.request.school }
    pub fn requested_program(&self) -> &TeamId { &self.request.program }
    pub fn source_parts(&mut self) -> StoreResult<(&RosterSourceEvidenceV1, &mut AttachmentReplayGrantV1)> {
        self.evidence.require_original_grant(&mut self.grant)?;
        Ok((&self.evidence, &mut self.grant))
    }
}

impl Store {
    pub fn inspect_roster_source_issuance_v1(
        &self,
        request: RosterIssuanceInspectionRequestV1,
        mut grant: AttachmentReplayGrantV1,
    ) -> StoreResult<RosterIssuanceInspectionV1<'_>> {
        grant.ensure_available().map_err(super::source_error)?;
        let _guard = self.lock_appends();
        let result = inspect(self, request, &mut grant);
        match result {
            Ok((revision, store, input_token, authority_head, request, evidence)) => {
                Ok(RosterIssuanceInspectionV1 { revision, store, input_token, authority_head, request, evidence, grant })
            }
            Err(error) => { grant.poison(); Err(error) }
        }
    }
}

type Inspected<'s> = (StoreRevision<'s>, StoreIdentity, Vec<u8>, Vec<u8>, RosterIssuanceInspectionRequestV1, RosterSourceEvidenceV1);

fn inspect<'s>(store: &'s Store, request: RosterIssuanceInspectionRequestV1, grant: &mut AttachmentReplayGrantV1) -> StoreResult<Inspected<'s>> {
    let current = store.store_format()?;
    format::validate_format(&store.db.snapshot(), &store.meta, &store.journal, &store.receipts,
        &current, &mut RosterStartupValidationBudgetV1::new())?;
    if current.lifecycle_revision != Some(LIFECYCLE)
        || request.root.schema != RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
        || request.root.key.len() != 64
    { return Err(Error::EnrollmentRequired.into()); }
    let view = store.snapshot_locked();
    let continuation = grant.checkpoint().map_err(super::source_error)?;
    let held = view.read_retained_capture_root_v1(request.root.schema, &request.root.key, grant)?
        .ok_or(Error::Corrupt("provider root missing"))?;
    if held.root() != &request.root { return Err(Error::InputMoved.into()); }
    let rows = inventory::collect(&view, grant)?;
    grant.charge_bytes(2 * 512).map_err(super::source_error)?;
    grant.charge_steps(8).map_err(super::source_error)?;
    let input_token = view.attachment_input_revision_bytes()?;
    let snapshot = store.db.snapshot();
    let length = snapshot.size_of(&store.meta, HEAD)
        .map_err(|source| crate::StoreError::Read { source })?
        .ok_or(Error::Corrupt("authority head missing"))?;
    let length = usize::try_from(length).map_err(|_| Error::BudgetExceeded)?;
    grant.charge_bytes(length).map_err(super::source_error)?;
    if length > codec::MAX_BYTES { return Err(Error::BudgetExceeded.into()); }
    let raw = snapshot.get(&store.meta, HEAD)
        .map_err(|source| crate::StoreError::Read { source })?
        .ok_or(Error::Corrupt("authority head missing"))?;
    if raw.len() != length { return Err(Error::Corrupt("authority head length").into()); }
    continuation.require(grant).map_err(super::source_error)?;
    let identity = view.store_identity()?;
    let evidence = RosterSourceEvidenceV1::new(identity.clone(), view.sequence(), request.school.clone(), request.program.clone(), held, rows, continuation);
    Ok((view.revision(), identity, input_token, raw.to_vec(), request, evidence))
}

#[cfg(test)]
mod tests;
