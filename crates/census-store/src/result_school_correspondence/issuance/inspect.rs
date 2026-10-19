use super::*;
use census_domain::model::StoreIdentity;

mod metadata;

pub(super) struct Inspected<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) store: StoreIdentity,
    pub(super) inventory: ResultSchoolInventoryV1,
    pub(super) input_token: ResultSchoolDigestV1,
    pub(super) public_head: AttachmentPublicHeadV1,
}

pub(super) fn inspect(
    store: &Store,
    mut grant: AttachmentReplayGrantV1,
) -> StoreResult<(Inspected<'_>, AttachmentReplayGrantV1)> {
    let inspected = guarded(&mut grant, |grant| {
        let format = store.store_format()?;
        if !super::super::retains_school_enrollment(format.lifecycle_revision)
            || format.schema_version != Some(2)
            || format.key_format != Some(1)
            || format.migration_target.is_some()
        {
            return Err(Error::EnrollmentRequired.into());
        }
        let view = store.snapshot_locked();
        let meta = metadata::bounded(&view, grant)?;
        let (input_token, public_head) = super::super::enrollment::inspect::inputs(&view, grant)?;
        Ok(Inspected {
            revision: meta.revision,
            store: meta.store,
            inventory: meta.inventory,
            input_token,
            public_head,
        })
    })?;
    Ok((inspected, grant))
}
