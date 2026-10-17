use super::{approval, PreparedRosterEnrollmentV1, RosterEnrollmentCommandV1};
use super::super::{codec, format, RosterSourceIssuanceStoreError as Error};
use super::super::{RosterStartupValidationBudgetV1, LIFECYCLE, OPERATION_PREFIX};
use crate::{Store, StoreResult, StoreRevision};

impl Store {
    pub fn prepare_roster_source_enrollment_v1<'s>(
        &'s self,
        expected: StoreRevision<'s>,
        command: RosterEnrollmentCommandV1,
    ) -> StoreResult<PreparedRosterEnrollmentV1<'s>> {
        let _guard = self.lock_appends();
        expected.ensure_origin(self, command.operation.as_str())?;
        validate_command(&command)?;
        let current = self.store_format()?;
        format::validate_format(&self.db.snapshot(), &self.meta, &self.journal, &self.receipts,
            &current, &mut RosterStartupValidationBudgetV1::new())?;
        if !matches!(current.lifecycle_revision, Some(7 | LIFECYCLE)) {
            return Err(Error::EnrollmentRequired.into());
        }
        if current.lifecycle_revision == Some(7) {
            expected.ensure_current(self, command.operation.as_str())?;
            require_input(self, &expected, &command)?;
        }
        let approval = approval(&command)?;
        let approval_digest = codec::digest(&approval);
        Ok(PreparedRosterEnrollmentV1 { revision: expected, command, approval, approval_digest })
    }
}

pub(in super::super) fn validate_command(command: &RosterEnrollmentCommandV1) -> StoreResult<()> {
    if command.version != 1 || command.operation.as_str().len() > 256
        || command.operation.as_str().starts_with(OPERATION_PREFIX)
        || command.input_token_digest.as_str().len() != 64
        || !command.input_token_digest.as_str().bytes().all(|byte| byte.is_ascii_hexdigit()
            && !byte.is_ascii_uppercase())
    {
        return Err(Error::Corrupt("enrollment command").into());
    }
    Ok(())
}

pub(super) fn require_input(
    store: &Store,
    revision: &StoreRevision<'_>,
    command: &RosterEnrollmentCommandV1,
) -> StoreResult<()> {
    let format = store.store_format()?;
    let ledger = crate::public_identity::validate(&store.db.snapshot(), &store.entities,
        &store.meta, &store.journal, &store.receipts, &format)?
        .ok_or(Error::EnrollmentRequired)?;
    ledger.require_version(census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityGraphV3)?;
    let view = store.snapshot_locked();
    let token = view.attachment_input_revision_bytes()?;
    if view.store_identity()? != command.store
        || revision.evidence_generation() != command.evidence_generation
        || revision.derived_generation() != command.derived_generation
        || codec::digest(&token) != command.input_token_digest.as_str()
    {
        return Err(Error::InputMoved.into());
    }
    Ok(())
}
