use super::super::{invalid, limits::ObjectLimits, ObjectRefV2, MAX_RECORD_BYTES};
use crate::read::JournalObject;
use crate::{StoreError, StoreResult};

pub(super) fn limits(
    object: &ObjectRefV2,
    maximum: ObjectLimits,
) -> StoreResult<crate::read::ContextReadLimits> {
    object.check_bounds(maximum)?;
    Ok(crate::read::ContextReadLimits {
        bytes: usize::try_from(object.bytes()).map_err(|_| StoreError::CounterOverflow)?,
        records: usize::try_from(object.records()).map_err(|_| StoreError::CounterOverflow)?,
        record_bytes: MAX_RECORD_BYTES,
    })
}

pub(super) fn check_manifest(reader: &JournalObject<'_>, object: &ObjectRefV2) -> StoreResult<()> {
    if reader.object_id() != object.root().as_str()
        || u64::try_from(reader.total_bytes()).map_err(|_| StoreError::CounterOverflow)?
            != u64::from(object.bytes())
        || u64::try_from(reader.record_count()).map_err(|_| StoreError::CounterOverflow)?
            != u64::from(object.records())
    {
        return Err(invalid("retained object manifest differs from reference"));
    }
    Ok(())
}
